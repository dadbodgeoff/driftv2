//! Intraprocedural taint analysis — within-function dataflow tracking.
//!
//! Phase 1 of taint analysis. Covers most common vulnerability patterns
//! by tracking taint within a single function body.
//! Performance target: <1ms per function.

use drift_core::types::collections::{FxHashMap, FxHashSet};

use crate::parsers::types::{CallSite, FunctionInfo, ParseResult};

use super::registry::TaintRegistry;
use super::types::*;

/// Analyze a single function for intraprocedural taint flows.
///
/// Tracks taint from sources through local variables to sinks,
/// applying sanitizers along the way.
pub fn analyze_intraprocedural(
    parse_result: &ParseResult,
    registry: &TaintRegistry,
) -> Vec<TaintFlow> {
    let mut flows = Vec::new();

    for func in &parse_result.functions {
        let func_flows = analyze_function(func, parse_result, registry);
        flows.extend(func_flows);
    }

    // Also analyze class methods
    for class in &parse_result.classes {
        for method in &class.methods {
            let method_flows = analyze_function(method, parse_result, registry);
            flows.extend(method_flows);
        }
    }

    flows
}

/// Analyze a single function for taint flows.
fn analyze_function(
    func: &FunctionInfo,
    parse_result: &ParseResult,
    registry: &TaintRegistry,
) -> Vec<TaintFlow> {
    let mut flows = Vec::new();
    let mut tainted_vars: FxHashMap<String, TaintLabel> = FxHashMap::default();
    let mut sanitized_vars: FxHashSet<String> = FxHashSet::default();
    let mut label_counter: u64 = 0;

    // Phase 1: Identify sources within this function's scope
    let func_sources = find_sources_in_function(func, parse_result, registry, &mut label_counter);

    // Mark source variables as tainted
    for source in &func_sources {
        tainted_vars.insert(source.expression.clone(), source.label.clone());
    }

    // Phase 2: Identify tainted parameters
    for param in &func.parameters {
        if let Some(source_pattern) = registry.match_source(&param.name) {
            let label = TaintLabel::new(label_counter, source_pattern.source_type);
            label_counter += 1;
            tainted_vars.insert(param.name.clone(), label);
        }
    }

    // Phase 3: Track taint through call sites within this function
    let func_calls = find_calls_in_function(func, parse_result);

    // Sort calls by line number for sequential analysis
    let mut sorted_calls: Vec<&CallSite> = func_calls.into_iter().collect();
    sorted_calls.sort_by_key(|c| c.line);

    let mut sanitizers_applied = Vec::new();

    for call in &sorted_calls {
        let callee_name = &call.callee_name;
        let full_name = if let Some(ref receiver) = call.receiver {
            format!("{}.{}", receiver, callee_name)
        } else {
            callee_name.clone()
        };

        // Check if this call is a sanitizer
        if let Some(sanitizer_pattern) = registry.match_sanitizer(&full_name) {
            // Mark receiver/arguments as sanitized
            if let Some(ref receiver) = call.receiver {
                sanitized_vars.insert(receiver.clone());
            }
            sanitizers_applied.push(TaintSanitizer {
                file: parse_result.file.clone(),
                line: call.line,
                expression: full_name.clone(),
                sanitizer_type: sanitizer_pattern.sanitizer_type,
                labels_sanitized: sanitizer_pattern.protects_against.clone(),
            });
            continue;
        }

        // Check if this call is a sink
        if let Some(sink_pattern) = registry.match_sink(&full_name) {
            // Check if any tainted variable flows into this sink
            let is_tainted = check_taint_reaches_sink(
                &tainted_vars,
                &sanitized_vars,
                call,
            );

            if is_tainted {
                let is_sanitized = check_sanitized_for_sink(
                    &sanitizers_applied,
                    &sink_pattern.sink_type,
                );

                // Find the source that originated this taint
                let source = func_sources.first().cloned().unwrap_or_else(|| {
                    TaintSource {
                        file: parse_result.file.clone(),
                        line: func.line,
                        column: 0,
                        expression: "unknown_source".to_string(),
                        source_type: SourceType::UserInput,
                        label: TaintLabel::new(0, SourceType::UserInput),
                    }
                });

                let sink = TaintSink {
                    file: parse_result.file.clone(),
                    line: call.line,
                    column: call.column,
                    expression: full_name.clone(),
                    sink_type: sink_pattern.sink_type,
                    required_sanitizers: sink_pattern.required_sanitizers.clone(),
                };

                let path = build_intraprocedural_path(&source, &sink, func);

                flows.push(TaintFlow {
                    source,
                    sink,
                    path,
                    is_sanitized,
                    sanitizers_applied: if is_sanitized {
                        sanitizers_applied.clone()
                    } else {
                        Vec::new()
                    },
                    cwe_id: sink_pattern.sink_type.cwe_id(),
                    confidence: if is_sanitized { 0.3 } else { 0.85 },
                });
            }
        }
    }

    flows
}

/// Find taint sources within a function's scope.
fn find_sources_in_function(
    func: &FunctionInfo,
    parse_result: &ParseResult,
    registry: &TaintRegistry,
    label_counter: &mut u64,
) -> Vec<TaintSource> {
    let mut sources = Vec::new();

    // Check function parameters for source patterns
    for param in &func.parameters {
        if let Some(source_pattern) = registry.match_source(&param.name) {
            let label = TaintLabel::new(*label_counter, source_pattern.source_type);
            *label_counter += 1;
            sources.push(TaintSource {
                file: parse_result.file.clone(),
                line: func.line,
                column: 0,
                expression: param.name.clone(),
                source_type: source_pattern.source_type,
                label,
            });
        }
    }

    // Check call sites within function scope for source patterns
    for call in &parse_result.call_sites {
        if call.line >= func.line && call.line <= func.end_line {
            let full_name = if let Some(ref receiver) = call.receiver {
                format!("{}.{}", receiver, call.callee_name)
            } else {
                call.callee_name.clone()
            };

            if let Some(source_pattern) = registry.match_source(&full_name) {
                let label = TaintLabel::new(*label_counter, source_pattern.source_type);
                *label_counter += 1;
                sources.push(TaintSource {
                    file: parse_result.file.clone(),
                    line: call.line,
                    column: call.column,
                    expression: full_name,
                    source_type: source_pattern.source_type,
                    label,
                });
            }
        }
    }

    sources
}

/// Find call sites within a function's line range.
fn find_calls_in_function<'a>(
    func: &FunctionInfo,
    parse_result: &'a ParseResult,
) -> Vec<&'a CallSite> {
    parse_result
        .call_sites
        .iter()
        .filter(|c| c.line >= func.line && c.line <= func.end_line)
        .collect()
}

/// Check if tainted data reaches a sink call.
fn check_taint_reaches_sink(
    tainted_vars: &FxHashMap<String, TaintLabel>,
    sanitized_vars: &FxHashSet<String>,
    call: &CallSite,
) -> bool {
    // Check if receiver is tainted
    if let Some(ref receiver) = call.receiver {
        if tainted_vars.contains_key(receiver) && !sanitized_vars.contains(receiver) {
            return true;
        }
    }

    // If there are any tainted variables in scope, conservatively assume taint reaches
    // (proper dataflow would track through assignments, but this is a sound approximation)
    !tainted_vars.is_empty()
}

/// Check if the appropriate sanitizer has been applied for a given sink type.
fn check_sanitized_for_sink(
    sanitizers: &[TaintSanitizer],
    sink_type: &SinkType,
) -> bool {
    sanitizers.iter().any(|s| s.labels_sanitized.contains(sink_type))
}

/// Build an intraprocedural path from source to sink.
fn build_intraprocedural_path(
    source: &TaintSource,
    sink: &TaintSink,
    func: &FunctionInfo,
) -> Vec<TaintHop> {
    let mut path = Vec::new();

    // Source hop
    path.push(TaintHop {
        file: source.file.clone(),
        line: source.line,
        column: source.column,
        function: func.name.clone(),
        description: format!("Taint introduced from {}", source.source_type.name()),
    });

    // If source and sink are on different lines, add intermediate hop
    if source.line != sink.line {
        path.push(TaintHop {
            file: sink.file.clone(),
            line: sink.line,
            column: sink.column,
            function: func.name.clone(),
            description: format!("Taint flows to {} sink", sink.sink_type.name()),
        });
    }

    path
}
