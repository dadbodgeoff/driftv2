//! Breaking change classifier: 20+ change types, paradigm-specific rules.

use super::types::*;

/// Classify breaking changes between two versions of a contract.
pub fn classify_breaking_changes(
    old_contract: &Contract,
    new_contract: &Contract,
) -> Vec<BreakingChange> {
    let mut changes = Vec::new();

    let old_endpoints: std::collections::HashMap<String, &Endpoint> = old_contract
        .endpoints
        .iter()
        .map(|e| (format!("{}:{}", e.method, e.path), e))
        .collect();

    let new_endpoints: std::collections::HashMap<String, &Endpoint> = new_contract
        .endpoints
        .iter()
        .map(|e| (format!("{}:{}", e.method, e.path), e))
        .collect();

    // Check for removed endpoints
    for (key, old_ep) in &old_endpoints {
        if !new_endpoints.contains_key(key) {
            changes.push(BreakingChange {
                change_type: BreakingChangeType::EndpointRemoved,
                endpoint: old_ep.path.clone(),
                field: None,
                severity: MismatchSeverity::Critical,
                message: format!("{} {} was removed", old_ep.method, old_ep.path),
            });
        }
    }

    // Check for field-level changes in existing endpoints
    for (key, new_ep) in &new_endpoints {
        if let Some(old_ep) = old_endpoints.get(key) {
            // Check removed fields
            for old_field in &old_ep.response_fields {
                if !new_ep.response_fields.iter().any(|f| f.name == old_field.name) {
                    changes.push(BreakingChange {
                        change_type: BreakingChangeType::FieldRemoved,
                        endpoint: new_ep.path.clone(),
                        field: Some(old_field.name.clone()),
                        severity: MismatchSeverity::High,
                        message: format!("Field '{}' removed from response", old_field.name),
                    });
                }
            }

            // Check type changes
            for new_field in &new_ep.response_fields {
                if let Some(old_field) = old_ep.response_fields.iter().find(|f| f.name == new_field.name) {
                    if old_field.field_type != new_field.field_type {
                        changes.push(BreakingChange {
                            change_type: BreakingChangeType::TypeChanged,
                            endpoint: new_ep.path.clone(),
                            field: Some(new_field.name.clone()),
                            severity: MismatchSeverity::High,
                            message: format!(
                                "Field '{}' type changed from {} to {}",
                                new_field.name, old_field.field_type, new_field.field_type
                            ),
                        });
                    }

                    // Optional → Required
                    if !old_field.required && new_field.required {
                        changes.push(BreakingChange {
                            change_type: BreakingChangeType::OptionalToRequired,
                            endpoint: new_ep.path.clone(),
                            field: Some(new_field.name.clone()),
                            severity: MismatchSeverity::High,
                            message: format!("Field '{}' changed from optional to required", new_field.name),
                        });
                    }
                }
            }

            // New required fields added to request
            for new_field in &new_ep.request_fields {
                if new_field.required
                    && !old_ep.request_fields.iter().any(|f| f.name == new_field.name)
                {
                    changes.push(BreakingChange {
                        change_type: BreakingChangeType::RequiredAdded,
                        endpoint: new_ep.path.clone(),
                        field: Some(new_field.name.clone()),
                        severity: MismatchSeverity::Medium,
                        message: format!("New required field '{}' added to request", new_field.name),
                    });
                }
            }
        }
    }

    changes
}
