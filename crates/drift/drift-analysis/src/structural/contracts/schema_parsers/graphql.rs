//! GraphQL SDL schema parser.

use super::SchemaParser;
use crate::structural::contracts::types::*;

/// Parses GraphQL Schema Definition Language (SDL).
pub struct GraphqlParser;

impl SchemaParser for GraphqlParser {
    fn parse(&self, content: &str, file_path: &str) -> Vec<Contract> {
        let mut endpoints = Vec::new();

        // Parse type definitions for Query, Mutation, Subscription
        let operation_types = ["Query", "Mutation", "Subscription"];

        for op_type in &operation_types {
            let type_pattern = format!("type {} {{", op_type);
            if let Some(start) = content.find(&type_pattern) {
                let block = extract_block(content, start + type_pattern.len());
                let fields = parse_graphql_fields(&block, file_path);
                for field in fields {
                    endpoints.push(Endpoint {
                        method: op_type.to_string(),
                        path: field.0,
                        request_fields: field.1,
                        response_fields: field.2,
                        file: file_path.to_string(),
                        line: 0,
                    });
                }
            }
        }

        if endpoints.is_empty() {
            return vec![];
        }

        vec![Contract {
            id: format!("graphql:{}", file_path),
            paradigm: Paradigm::GraphQL,
            endpoints,
            source_file: file_path.to_string(),
            framework: "graphql".to_string(),
            confidence: 0.90,
        }]
    }

    fn extensions(&self) -> &[&str] {
        &["graphql", "gql"]
    }

    fn schema_type(&self) -> &str {
        "graphql"
    }
}

/// Extract a brace-delimited block.
fn extract_block(content: &str, start: usize) -> String {
    let bytes = content.as_bytes();
    let mut depth = 1;
    let mut end = start;

    for (i, &b) in bytes[start..].iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    content[start..end].to_string()
}

/// Parse GraphQL field definitions into (name, args, return_fields).
fn parse_graphql_fields(
    block: &str,
    _file_path: &str,
) -> Vec<(String, Vec<FieldSpec>, Vec<FieldSpec>)> {
    let mut results = Vec::new();

    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Parse: fieldName(arg1: Type!, arg2: Type): ReturnType
        if let Some(colon_pos) = trimmed.rfind(':') {
            let before_return = &trimmed[..colon_pos];
            let return_type = trimmed[colon_pos + 1..].trim().trim_end_matches('!');

            let (name, args) = if let Some(paren_start) = before_return.find('(') {
                let name = before_return[..paren_start].trim().to_string();
                let args_str = &before_return[paren_start + 1..];
                let args_str = args_str.trim_end_matches(')');
                let args = parse_graphql_args(args_str);
                (name, args)
            } else {
                (before_return.trim().to_string(), vec![])
            };

            if !name.is_empty() {
                let response_fields = vec![FieldSpec {
                    name: "result".to_string(),
                    field_type: return_type.trim_start_matches('[').trim_end_matches(']').to_string(),
                    required: trimmed.ends_with('!'),
                    nullable: !trimmed.ends_with('!'),
                }];

                results.push((name, args, response_fields));
            }
        }
    }

    results
}

fn parse_graphql_args(args_str: &str) -> Vec<FieldSpec> {
    args_str
        .split(',')
        .filter_map(|arg| {
            let parts: Vec<&str> = arg.split(':').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().to_string();
                let type_str = parts[1].trim();
                let required = type_str.ends_with('!');
                let field_type = type_str.trim_end_matches('!').to_string();
                Some(FieldSpec {
                    name,
                    field_type,
                    required,
                    nullable: !required,
                })
            } else {
                None
            }
        })
        .collect()
}
