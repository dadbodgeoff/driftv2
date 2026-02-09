//! 6 resolution strategies: Direct, Method, Constructor, Callback, Dynamic, External.
//! First match wins — strategies are tried in order of decreasing confidence.

use drift_core::types::collections::FxHashMap;

use crate::parsers::types::{CallSite, ImportInfo};

use super::types::Resolution;

/// Attempt to resolve a call site to a callee function key.
///
/// Tries strategies in order: SameFile → MethodCall → ImportBased → ExportBased → Fuzzy.
/// Returns the callee key and the resolution strategy used.
pub fn resolve_call(
    call_site: &CallSite,
    caller_file: &str,
    imports: &[ImportInfo],
    name_index: &FxHashMap<String, Vec<String>>,
    qualified_index: &FxHashMap<String, String>,
    export_index: &FxHashMap<String, Vec<String>>,
) -> Option<(String, Resolution)> {
    // Strategy 1: Same-file direct call (confidence 0.95)
    if let Some(result) = resolve_same_file(call_site, caller_file, name_index) {
        return Some((result, Resolution::SameFile));
    }

    // Strategy 2: Method call on known receiver (confidence 0.90)
    if let Some(result) = resolve_method_call(call_site, qualified_index) {
        return Some((result, Resolution::MethodCall));
    }

    // Strategy 3: Import-based resolution (confidence 0.75)
    if let Some(result) = resolve_import_based(call_site, imports, name_index) {
        return Some((result, Resolution::ImportBased));
    }

    // Strategy 4: Export-based cross-module (confidence 0.60)
    if let Some(result) = resolve_export_based(call_site, export_index) {
        return Some((result, Resolution::ExportBased));
    }

    // Strategy 5: Fuzzy name matching (confidence 0.40)
    if let Some(result) = resolve_fuzzy(call_site, name_index) {
        return Some((result, Resolution::Fuzzy));
    }

    None
}

/// Same-file resolution: callee is in the same file as caller.
fn resolve_same_file(
    call_site: &CallSite,
    caller_file: &str,
    name_index: &FxHashMap<String, Vec<String>>,
) -> Option<String> {
    let callee_name = &call_site.callee_name;
    if let Some(keys) = name_index.get(callee_name) {
        let same_file_key = format!("{}::{}", caller_file, callee_name);
        if keys.contains(&same_file_key) {
            return Some(same_file_key);
        }
    }
    None
}

/// Method call resolution: receiver.method() → Class.method qualified name.
fn resolve_method_call(
    call_site: &CallSite,
    qualified_index: &FxHashMap<String, String>,
) -> Option<String> {
    if let Some(ref receiver) = call_site.receiver {
        let qualified = format!("{}.{}", receiver, call_site.callee_name);
        if let Some(key) = qualified_index.get(&qualified) {
            return Some(key.clone());
        }
    }
    None
}

/// Import-based resolution: callee is imported from another module.
fn resolve_import_based(
    call_site: &CallSite,
    imports: &[ImportInfo],
    name_index: &FxHashMap<String, Vec<String>>,
) -> Option<String> {
    let callee_name = &call_site.callee_name;

    for import in imports {
        for spec in &import.specifiers {
            let effective_name = spec.alias.as_deref().unwrap_or(&spec.name);
            if effective_name == callee_name {
                // Find the function in the source module
                if let Some(keys) = name_index.get(&spec.name) {
                    // Prefer keys from the import source
                    for key in keys {
                        if key.contains(&import.source) {
                            return Some(key.clone());
                        }
                    }
                    // Fall back to first match
                    return keys.first().cloned();
                }
            }
        }
    }
    None
}

/// Export-based resolution: callee is exported from some module.
fn resolve_export_based(
    call_site: &CallSite,
    export_index: &FxHashMap<String, Vec<String>>,
) -> Option<String> {
    if let Some(keys) = export_index.get(&call_site.callee_name) {
        // If there's exactly one exported function with this name, use it
        if keys.len() == 1 {
            return Some(keys[0].clone());
        }
    }
    None
}

/// Fuzzy name matching: last resort, lowest confidence.
fn resolve_fuzzy(
    call_site: &CallSite,
    name_index: &FxHashMap<String, Vec<String>>,
) -> Option<String> {
    if let Some(keys) = name_index.get(&call_site.callee_name) {
        // Only use fuzzy if there's exactly one match globally
        if keys.len() == 1 {
            return Some(keys[0].clone());
        }
    }
    None
}

/// Resolve a constructor call (new ClassName()).
pub fn resolve_constructor(
    class_name: &str,
    qualified_index: &FxHashMap<String, String>,
    name_index: &FxHashMap<String, Vec<String>>,
) -> Option<(String, Resolution)> {
    // Try qualified constructor name
    let constructor_names = [
        format!("{}.constructor", class_name),
        format!("{}.__init__", class_name),
        format!("{}.new", class_name),
        format!("{}.init", class_name),
    ];

    for qn in &constructor_names {
        if let Some(key) = qualified_index.get(qn) {
            return Some((key.clone(), Resolution::MethodCall));
        }
    }

    // Fall back to class name as function
    if let Some(keys) = name_index.get(class_name) {
        if keys.len() == 1 {
            return Some((keys[0].clone(), Resolution::Fuzzy));
        }
    }

    None
}
