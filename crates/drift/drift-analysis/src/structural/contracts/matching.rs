//! BE↔FE matching via path similarity + schema compatibility scoring.

use super::types::*;

/// Match backend endpoints to frontend consumers.
pub fn match_contracts(
    backend: &[Endpoint],
    frontend: &[Endpoint],
) -> Vec<ContractMatch> {
    let mut matches = Vec::new();

    for be in backend {
        for fe in frontend {
            let confidence = compute_match_confidence(be, fe);
            if confidence >= 0.5 {
                let mismatches = detect_mismatches(be, fe);
                matches.push(ContractMatch {
                    backend: be.clone(),
                    frontend: fe.clone(),
                    confidence,
                    mismatches,
                });
            }
        }
    }

    // Sort by confidence descending
    matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    matches
}

/// Compute match confidence between a backend endpoint and frontend call.
fn compute_match_confidence(backend: &Endpoint, frontend: &Endpoint) -> f64 {
    let mut score = 0.0;
    let mut signals = 0.0;

    // Signal 1: Path similarity (highest weight)
    let path_sim = path_similarity(&backend.path, &frontend.path);
    score += path_sim * 3.0;
    signals += 3.0;

    // Signal 2: Method match
    if backend.method == frontend.method || frontend.method == "GET" || frontend.method == "ANY" {
        score += 1.0;
    }
    signals += 1.0;

    // Signal 3: Field overlap
    if !backend.response_fields.is_empty() && !frontend.request_fields.is_empty() {
        let overlap = field_overlap(&backend.response_fields, &frontend.request_fields);
        score += overlap;
    }
    signals += 1.0;

    if signals == 0.0 { 0.0 } else { score / signals }
}

/// Compute path similarity (normalized Levenshtein-like).
fn path_similarity(a: &str, b: &str) -> f64 {
    let a_norm = normalize_path(a);
    let b_norm = normalize_path(b);

    if a_norm == b_norm {
        return 1.0;
    }

    let a_parts: Vec<&str> = a_norm.split('/').filter(|s| !s.is_empty()).collect();
    let b_parts: Vec<&str> = b_norm.split('/').filter(|s| !s.is_empty()).collect();

    if a_parts.is_empty() || b_parts.is_empty() {
        return 0.0;
    }

    let max_len = a_parts.len().max(b_parts.len());
    let matching = a_parts.iter().zip(b_parts.iter())
        .filter(|(a, b)| {
            a == b || a.starts_with(':') || b.starts_with(':')
                || a.starts_with('{') || b.starts_with('{')
        })
        .count();

    matching as f64 / max_len as f64
}

fn normalize_path(path: &str) -> String {
    path.trim_end_matches('/')
        .replace("//", "/")
        .to_lowercase()
}

fn field_overlap(a: &[FieldSpec], b: &[FieldSpec]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_names: std::collections::HashSet<&str> = a.iter().map(|f| f.name.as_str()).collect();
    let b_names: std::collections::HashSet<&str> = b.iter().map(|f| f.name.as_str()).collect();
    let intersection = a_names.intersection(&b_names).count();
    let union = a_names.union(&b_names).count();
    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

fn detect_mismatches(backend: &Endpoint, frontend: &Endpoint) -> Vec<ContractMismatch> {
    let mut mismatches = Vec::new();

    // Check for missing fields
    for be_field in &backend.response_fields {
        let fe_match = frontend.request_fields.iter().find(|f| f.name == be_field.name);
        if fe_match.is_none() && be_field.required {
            mismatches.push(ContractMismatch {
                backend_endpoint: backend.path.clone(),
                frontend_call: frontend.path.clone(),
                mismatch_type: MismatchType::FieldMissing,
                severity: MismatchSeverity::High,
                message: format!("Required field '{}' not consumed by frontend", be_field.name),
            });
        }
    }

    mismatches
}
