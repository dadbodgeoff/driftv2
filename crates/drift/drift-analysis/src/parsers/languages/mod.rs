//! Per-language parser implementations.

pub mod csharp;
pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust_lang;
pub mod typescript;

use std::path::Path;
use std::time::Instant;

use drift_core::errors::ParseError;
use smallvec::SmallVec;
use tree_sitter::{Node, Parser, Query, QueryCursor};

use super::error_tolerant::count_errors;
use super::types::*;
use crate::scanner::language_detect::Language;
use crate::scanner::hasher::hash_content;

/// Shared parsing logic used by all language parsers via the `define_parser!` macro.
pub fn parse_with_language(
    source: &[u8],
    path: &Path,
    language: Language,
    ts_language: tree_sitter::Language,
) -> Result<ParseResult, ParseError> {
    let start = Instant::now();
    let file_str = path.to_string_lossy().to_string();
    let content_hash = hash_content(source);

    // Parse with tree-sitter
    let mut parser = Parser::new();
    parser.set_language(&ts_language).map_err(|e| ParseError::GrammarNotFound {
        language: language.name().to_string(),
    })?;

    let tree = parser.parse(source, None).ok_or_else(|| ParseError::TreeSitterError {
        path: path.to_path_buf(),
        message: "tree-sitter returned None".to_string(),
    })?;

    let root = tree.root_node();
    let (error_count, error_ranges) = count_errors(root);

    // Extract structural elements
    let source_str = std::str::from_utf8(source).unwrap_or("");
    let mut result = ParseResult {
        file: file_str.clone(),
        language,
        content_hash,
        has_errors: error_count > 0,
        error_count,
        error_ranges,
        ..Default::default()
    };

    // Extract functions, classes, imports, exports from the tree
    extract_structure(&mut result, root, source, &file_str);
    extract_calls(&mut result, root, source, &file_str);

    result.parse_time_us = start.elapsed().as_micros() as u64;
    Ok(result)
}

/// Extract structural elements (functions, classes, imports, exports) from the AST.
fn extract_structure(result: &mut ParseResult, root: Node, source: &[u8], file: &str) {
    let mut cursor = root.walk();
    extract_node_recursive(result, &mut cursor, source, file, 0);
}

fn extract_node_recursive(
    result: &mut ParseResult,
    cursor: &mut tree_sitter::TreeCursor,
    source: &[u8],
    file: &str,
    depth: usize,
) {
    let node = cursor.node();
    let kind = node.kind();

    match kind {
        // Functions
        "function_declaration" | "function_definition" | "function_item"
        | "method_declaration" | "method_definition" | "method" | "singleton_method" => {
            if let Some(func) = extract_function(node, source, file) {
                result.functions.push(func);
            }
        }
        // Arrow functions (JS/TS)
        "arrow_function" => {
            if let Some(func) = extract_arrow_function(node, source, file) {
                result.functions.push(func);
            }
        }
        // Classes
        "class_declaration" | "class_definition" | "class" => {
            if let Some(class) = extract_class(node, source, file, result.language) {
                result.classes.push(class);
            }
        }
        // Interfaces
        "interface_declaration" => {
            if let Some(class) = extract_interface(node, source, file) {
                result.classes.push(class);
            }
        }
        // Structs (Rust, Go)
        "struct_item" => {
            if let Some(class) = extract_struct(node, source, file) {
                result.classes.push(class);
            }
        }
        // Enums
        "enum_item" | "enum_declaration" => {
            if let Some(class) = extract_enum(node, source, file) {
                result.classes.push(class);
            }
        }
        // Traits (Rust)
        "trait_item" => {
            if let Some(class) = extract_trait(node, source, file) {
                result.classes.push(class);
            }
        }
        // Imports
        "import_statement" | "import_declaration" | "import_from_statement"
        | "use_declaration" | "using_directive" | "import_header" => {
            if let Some(import) = extract_import(node, source, file) {
                result.imports.push(import);
            }
        }
        // Exports
        "export_statement" | "export_declaration" => {
            if let Some(export) = extract_export(node, source, file) {
                result.exports.push(export);
            }
        }
        // Namespace/Package
        "package_declaration" | "package_clause" | "package_header"
        | "namespace_declaration" | "namespace_definition" => {
            result.namespace = extract_text_from_node(node, source);
        }
        _ => {}
    }

    // Recurse into children
    if depth < 50 && cursor.goto_first_child() {
        loop {
            extract_node_recursive(result, cursor, source, file, depth + 1);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Extract call sites, decorators, literals from the AST.
fn extract_calls(result: &mut ParseResult, root: Node, source: &[u8], file: &str) {
    let mut cursor = root.walk();
    extract_calls_recursive(result, &mut cursor, source, file, 0);
}

fn extract_calls_recursive(
    result: &mut ParseResult,
    cursor: &mut tree_sitter::TreeCursor,
    source: &[u8],
    file: &str,
    depth: usize,
) {
    let node = cursor.node();
    let kind = node.kind();

    match kind {
        "call_expression" | "call" | "method_invocation" | "invocation_expression"
        | "function_call_expression" | "member_call_expression" => {
            if let Some(call) = extract_call_site(node, source, file) {
                result.call_sites.push(call);
            }
        }
        "decorator" | "attribute" | "attribute_item" | "annotation"
        | "marker_annotation" => {
            if let Some(dec) = extract_decorator(node, source) {
                result.decorators.push(dec);
            }
        }
        "string" | "string_literal" | "interpreted_string_literal"
        | "raw_string_literal" | "template_string" => {
            if let Some(lit) = extract_string_literal(node, source, file) {
                result.string_literals.push(lit);
            }
        }
        "number" | "integer" | "float" | "integer_literal" | "float_literal"
        | "int_literal" | "decimal_integer_literal" | "decimal_floating_point_literal"
        | "real_literal" | "numeric_literal" => {
            if let Some(lit) = extract_numeric_literal(node, source, file) {
                result.numeric_literals.push(lit);
            }
        }
        "try_statement" | "try_expression" => {
            result.error_handling.push(ErrorHandlingInfo {
                kind: ErrorHandlingKind::TryCatch,
                file: file.to_string(),
                line: node.start_position().row as u32,
                end_line: node.end_position().row as u32,
                range: Range::from_ts_node(&node),
                caught_type: None,
                has_body: true,
                function_scope: None,
            });
        }
        "throw_statement" | "throw" | "raise_statement" | "raise" => {
            result.error_handling.push(ErrorHandlingInfo {
                kind: ErrorHandlingKind::Throw,
                file: file.to_string(),
                line: node.start_position().row as u32,
                end_line: node.end_position().row as u32,
                range: Range::from_ts_node(&node),
                caught_type: None,
                has_body: false,
                function_scope: None,
            });
        }
        "begin" => {
            result.error_handling.push(ErrorHandlingInfo {
                kind: ErrorHandlingKind::Rescue,
                file: file.to_string(),
                line: node.start_position().row as u32,
                end_line: node.end_position().row as u32,
                range: Range::from_ts_node(&node),
                caught_type: None,
                has_body: true,
                function_scope: None,
            });
        }
        "defer_statement" => {
            result.error_handling.push(ErrorHandlingInfo {
                kind: ErrorHandlingKind::Defer,
                file: file.to_string(),
                line: node.start_position().row as u32,
                end_line: node.end_position().row as u32,
                range: Range::from_ts_node(&node),
                caught_type: None,
                has_body: true,
                function_scope: None,
            });
        }
        _ => {}
    }

    if depth < 50 && cursor.goto_first_child() {
        loop {
            extract_calls_recursive(result, cursor, source, file, depth + 1);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

// ---- Extraction helpers ----

fn extract_function(node: Node, source: &[u8], file: &str) -> Option<FunctionInfo> {
    let name = find_child_text(&node, source, &["identifier", "property_identifier",
        "field_identifier", "name", "simple_identifier"])?;
    let body = node.child_by_field_name("body");
    let body_text = body.map(|b| node_text(b, source)).unwrap_or_default();
    let params_text = node.child_by_field_name("parameters")
        .map(|p| node_text(p, source))
        .unwrap_or_default();
    let return_type = node.child_by_field_name("return_type")
        .or_else(|| node.child_by_field_name("type"))
        .map(|t| node_text(t, source));

    let sig_return = return_type.as_deref().unwrap_or("");
    let sig_hash = hash_content(format!("{}({}){}", name, params_text, sig_return).as_bytes());

    Some(FunctionInfo {
        name: name.clone(),
        qualified_name: None,
        file: file.to_string(),
        line: node.start_position().row as u32,
        column: node.start_position().column as u32,
        end_line: node.end_position().row as u32,
        parameters: extract_parameters(node, source),
        return_type,
        generic_params: SmallVec::new(),
        visibility: Visibility::Public,
        is_exported: false,
        is_async: has_child_kind(&node, "async"),
        is_generator: has_child_kind(&node, "generator") || node.kind().contains("generator"),
        is_abstract: has_child_kind(&node, "abstract"),
        range: Range::from_ts_node(&node),
        decorators: Vec::new(),
        doc_comment: None,
        body_hash: hash_content(body_text.as_bytes()),
        signature_hash: sig_hash,
    })
}

fn extract_arrow_function(node: Node, source: &[u8], file: &str) -> Option<FunctionInfo> {
    // Arrow functions may be assigned to a variable
    let name = node.parent()
        .and_then(|p| {
            if p.kind() == "variable_declarator" || p.kind() == "lexical_declaration" {
                find_child_text(&p, source, &["identifier"])
            } else if p.kind() == "pair" || p.kind() == "property" {
                find_child_text(&p, source, &["property_identifier", "identifier"])
            } else {
                None
            }
        })
        .unwrap_or_else(|| "<anonymous>".to_string());

    let body = node.child_by_field_name("body");
    let body_text = body.map(|b| node_text(b, source)).unwrap_or_default();

    Some(FunctionInfo {
        name,
        qualified_name: None,
        file: file.to_string(),
        line: node.start_position().row as u32,
        column: node.start_position().column as u32,
        end_line: node.end_position().row as u32,
        parameters: extract_parameters(node, source),
        return_type: None,
        generic_params: SmallVec::new(),
        visibility: Visibility::Public,
        is_exported: false,
        is_async: node.parent().is_some_and(|p| has_child_kind(&p, "async")),
        is_generator: false,
        is_abstract: false,
        range: Range::from_ts_node(&node),
        decorators: Vec::new(),
        doc_comment: None,
        body_hash: hash_content(body_text.as_bytes()),
        signature_hash: 0,
    })
}

fn extract_class(node: Node, source: &[u8], file: &str, lang: Language) -> Option<ClassInfo> {
    let name = find_child_text(&node, source, &[
        "identifier", "type_identifier", "constant", "name",
    ])?;

    let extends = node.child_by_field_name("superclass")
        .or_else(|| find_child_by_kind(&node, "class_heritage"))
        .and_then(|n| extract_text_from_node(n, source));

    let mut methods = Vec::new();
    let mut properties = Vec::new();

    // Extract methods and properties from class body
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                match child.kind() {
                    "method_definition" | "method_declaration" | "method"
                    | "function_definition" | "function_item" => {
                        if let Some(mut func) = extract_function(child, source, file) {
                            func.qualified_name = Some(format!("{}.{}", name, func.name));
                            methods.push(func);
                        }
                    }
                    "public_field_definition" | "field_declaration" | "property_declaration" => {
                        if let Some(prop) = extract_property(child, source) {
                            properties.push(prop);
                        }
                    }
                    _ => {}
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    Some(ClassInfo {
        name,
        namespace: None,
        extends,
        implements: SmallVec::new(),
        generic_params: SmallVec::new(),
        is_exported: false,
        is_abstract: has_child_kind(&node, "abstract"),
        class_kind: ClassKind::Class,
        methods,
        properties,
        range: Range::from_ts_node(&node),
        decorators: Vec::new(),
    })
}

fn extract_interface(node: Node, source: &[u8], file: &str) -> Option<ClassInfo> {
    let name = find_child_text(&node, source, &["identifier", "type_identifier", "name"])?;
    Some(ClassInfo {
        name,
        namespace: None,
        extends: None,
        implements: SmallVec::new(),
        generic_params: SmallVec::new(),
        is_exported: false,
        is_abstract: true,
        class_kind: ClassKind::Interface,
        methods: Vec::new(),
        properties: Vec::new(),
        range: Range::from_ts_node(&node),
        decorators: Vec::new(),
    })
}

fn extract_struct(node: Node, source: &[u8], file: &str) -> Option<ClassInfo> {
    let name = find_child_text(&node, source, &["type_identifier", "identifier"])?;
    Some(ClassInfo {
        name,
        namespace: None,
        extends: None,
        implements: SmallVec::new(),
        generic_params: SmallVec::new(),
        is_exported: false,
        is_abstract: false,
        class_kind: ClassKind::Struct,
        methods: Vec::new(),
        properties: Vec::new(),
        range: Range::from_ts_node(&node),
        decorators: Vec::new(),
    })
}

fn extract_enum(node: Node, source: &[u8], file: &str) -> Option<ClassInfo> {
    let name = find_child_text(&node, source, &["type_identifier", "identifier", "name"])?;
    Some(ClassInfo {
        name,
        namespace: None,
        extends: None,
        implements: SmallVec::new(),
        generic_params: SmallVec::new(),
        is_exported: false,
        is_abstract: false,
        class_kind: ClassKind::Enum,
        methods: Vec::new(),
        properties: Vec::new(),
        range: Range::from_ts_node(&node),
        decorators: Vec::new(),
    })
}

fn extract_trait(node: Node, source: &[u8], file: &str) -> Option<ClassInfo> {
    let name = find_child_text(&node, source, &["type_identifier"])?;
    Some(ClassInfo {
        name,
        namespace: None,
        extends: None,
        implements: SmallVec::new(),
        generic_params: SmallVec::new(),
        is_exported: false,
        is_abstract: true,
        class_kind: ClassKind::Trait,
        methods: Vec::new(),
        properties: Vec::new(),
        range: Range::from_ts_node(&node),
        decorators: Vec::new(),
    })
}

fn extract_import(node: Node, source: &[u8], file: &str) -> Option<ImportInfo> {
    let text = node_text(node, source);
    Some(ImportInfo {
        source: text,
        specifiers: SmallVec::new(),
        is_type_only: false,
        file: file.to_string(),
        line: node.start_position().row as u32,
    })
}

fn extract_export(node: Node, source: &[u8], file: &str) -> Option<ExportInfo> {
    let name = find_child_text(&node, source, &["identifier", "type_identifier"]);
    let is_default = node_text(node, source).contains("default");
    Some(ExportInfo {
        name,
        is_default,
        is_type_only: false,
        source: None,
        file: file.to_string(),
        line: node.start_position().row as u32,
    })
}

fn extract_call_site(node: Node, source: &[u8], file: &str) -> Option<CallSite> {
    let (callee_name, receiver) = extract_call_target(node, source)?;
    let args = node.child_by_field_name("arguments");
    let arg_count = args.map(|a| {
        let mut count = 0u8;
        let mut c = a.walk();
        if c.goto_first_child() {
            loop {
                let child = c.node();
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    count = count.saturating_add(1);
                }
                if !c.goto_next_sibling() { break; }
            }
        }
        count
    }).unwrap_or(0);

    let is_await = node.parent().is_some_and(|p| p.kind() == "await_expression");

    Some(CallSite {
        callee_name,
        receiver,
        file: file.to_string(),
        line: node.start_position().row as u32,
        column: node.start_position().column as u32,
        argument_count: arg_count,
        is_await,
    })
}

fn extract_call_target(node: Node, source: &[u8]) -> Option<(String, Option<String>)> {
    // Try function field first
    if let Some(func) = node.child_by_field_name("function") {
        match func.kind() {
            "identifier" | "name" | "simple_identifier" => {
                return Some((node_text(func, source), None));
            }
            "member_expression" | "member_access_expression" | "selector_expression"
            | "field_expression" | "attribute" | "navigation_expression" => {
                let obj = func.child_by_field_name("object")
                    .or_else(|| func.child_by_field_name("operand"))
                    .map(|n| node_text(n, source));
                let prop = func.child_by_field_name("property")
                    .or_else(|| func.child_by_field_name("field"))
                    .or_else(|| func.child_by_field_name("name"))
                    .or_else(|| func.child_by_field_name("attribute"))
                    .map(|n| node_text(n, source));
                if let Some(method) = prop {
                    return Some((method, obj));
                }
            }
            _ => {}
        }
    }
    // Try method field (Java)
    if let Some(method) = node.child_by_field_name("name") {
        let obj = node.child_by_field_name("object").map(|n| node_text(n, source));
        return Some((node_text(method, source), obj));
    }
    // Try direct child identifier
    if let Some(name) = find_child_text(&node, source, &["identifier", "name", "simple_identifier"]) {
        return Some((name, None));
    }
    // Fallback: method field for member calls
    if let Some(method) = node.child_by_field_name("method") {
        return Some((node_text(method, source), None));
    }
    None
}

fn extract_decorator(node: Node, source: &[u8]) -> Option<DecoratorInfo> {
    let name = find_child_text(&node, source, &[
        "identifier", "name", "type_identifier", "call_expression",
    ]).unwrap_or_else(|| node_text(node, source));

    Some(DecoratorInfo {
        name,
        arguments: SmallVec::new(),
        raw_text: node_text(node, source),
        range: Range::from_ts_node(&node),
    })
}

fn extract_string_literal(node: Node, source: &[u8], file: &str) -> Option<StringLiteralInfo> {
    let text = node_text(node, source);
    // Strip quotes
    let value = text.trim_matches(|c| c == '"' || c == '\'' || c == '`').to_string();
    Some(StringLiteralInfo {
        value,
        context: StringContext::Unknown,
        file: file.to_string(),
        line: node.start_position().row as u32,
        column: node.start_position().column as u32,
        range: Range::from_ts_node(&node),
    })
}

fn extract_numeric_literal(node: Node, source: &[u8], file: &str) -> Option<NumericLiteralInfo> {
    let raw = node_text(node, source);
    let value = raw.replace('_', "").parse::<f64>().unwrap_or(0.0);
    Some(NumericLiteralInfo {
        value,
        raw,
        context: NumericContext::Unknown,
        file: file.to_string(),
        line: node.start_position().row as u32,
        column: node.start_position().column as u32,
        range: Range::from_ts_node(&node),
    })
}

fn extract_parameters(node: Node, source: &[u8]) -> SmallVec<[ParameterInfo; 4]> {
    let mut params = SmallVec::new();
    if let Some(param_list) = node.child_by_field_name("parameters") {
        let mut cursor = param_list.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                match child.kind() {
                    "required_parameter" | "optional_parameter" | "formal_parameter"
                    | "parameter" | "identifier" | "typed_parameter"
                    | "default_parameter" | "rest_parameter" | "spread_parameter" => {
                        let name = find_child_text(&child, source, &[
                            "identifier", "name", "simple_identifier",
                        ]).unwrap_or_else(|| node_text(child, source));
                        let type_ann = child.child_by_field_name("type")
                            .map(|t| node_text(t, source));
                        let default = child.child_by_field_name("value")
                            .or_else(|| child.child_by_field_name("default_value"))
                            .map(|d| node_text(d, source));
                        let is_rest = child.kind().contains("rest") || child.kind().contains("spread");
                        params.push(ParameterInfo {
                            name,
                            type_annotation: type_ann,
                            default_value: default,
                            is_rest,
                        });
                    }
                    _ => {}
                }
                if !cursor.goto_next_sibling() { break; }
            }
        }
    }
    params
}

fn extract_property(node: Node, source: &[u8]) -> Option<PropertyInfo> {
    let name = find_child_text(&node, source, &[
        "property_identifier", "identifier", "name", "field_identifier",
    ])?;
    Some(PropertyInfo {
        name,
        type_annotation: node.child_by_field_name("type").map(|t| node_text(t, source)),
        is_static: has_child_kind(&node, "static"),
        is_readonly: has_child_kind(&node, "readonly"),
        visibility: Visibility::Public,
    })
}

// ---- Utility functions ----

fn node_text(node: Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or("").to_string()
}

fn extract_text_from_node(node: Node, source: &[u8]) -> Option<String> {
    let text = node.utf8_text(source).ok()?;
    if text.is_empty() { None } else { Some(text.to_string()) }
}

fn find_child_text(node: &Node, source: &[u8], kinds: &[&str]) -> Option<String> {
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            if kinds.contains(&child.kind()) {
                let text = node_text(child, source);
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }
    // Also check named children via field names
    for kind in kinds {
        if let Some(child) = node.child_by_field_name(kind) {
            let text = node_text(child, source);
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

fn find_child_by_kind<'a>(node: &'a Node<'a>, kind: &str) -> Option<Node<'a>> {
    let child_count = node.child_count();
    for i in 0..child_count {
        if let Some(child) = node.child(i) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}

fn has_child_kind(node: &Node, kind: &str) -> bool {
    find_child_by_kind(node, kind).is_some()
}
