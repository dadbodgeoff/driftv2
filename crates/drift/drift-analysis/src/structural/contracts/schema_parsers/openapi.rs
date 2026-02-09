//! OpenAPI 3.0/3.1 schema parser.

use super::SchemaParser;
use crate::structural::contracts::types::*;

/// Parses OpenAPI 3.0/3.1 specifications (JSON/YAML).
pub struct OpenApiParser;

impl SchemaParser for OpenApiParser {
    fn parse(&self, content: &str, file_path: &str) -> Vec<Contract> {
        // Try JSON first, then YAML
        let value: Option<serde_json::Value> = serde_json::from_str(content)
            .ok()
            .or_else(|| serde_yaml::from_str(content).ok());

        let value = match value {
            Some(v) => v,
            None => return vec![],
        };

        let mut endpoints = Vec::new();

        if let Some(paths) = value.get("paths").and_then(|p| p.as_object()) {
            for (path, methods) in paths {
                if let Some(methods_obj) = methods.as_object() {
                    for (method, operation) in methods_obj {
                        let http_methods = ["get", "post", "put", "delete", "patch", "options", "head"];
                        if !http_methods.contains(&method.as_str()) {
                            continue;
                        }

                        let request_fields = extract_request_fields(operation);
                        let response_fields = extract_response_fields(operation);

                        endpoints.push(Endpoint {
                            method: method.to_uppercase(),
                            path: path.clone(),
                            request_fields,
                            response_fields,
                            file: file_path.to_string(),
                            line: 0,
                        });
                    }
                }
            }
        }

        if endpoints.is_empty() {
            return vec![];
        }

        vec![Contract {
            id: format!("openapi:{}", file_path),
            paradigm: Paradigm::Rest,
            endpoints,
            source_file: file_path.to_string(),
            framework: "openapi".to_string(),
            confidence: 0.95,
        }]
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml", "json"]
    }

    fn schema_type(&self) -> &str {
        "openapi"
    }
}

fn extract_request_fields(operation: &serde_json::Value) -> Vec<FieldSpec> {
    let mut fields = Vec::new();

    // Extract from parameters
    if let Some(params) = operation.get("parameters").and_then(|p| p.as_array()) {
        for param in params {
            let name = param.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let required = param.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
            let field_type = param
                .get("schema")
                .and_then(|s| s.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("string");

            if !name.is_empty() {
                fields.push(FieldSpec {
                    name: name.to_string(),
                    field_type: field_type.to_string(),
                    required,
                    nullable: false,
                });
            }
        }
    }

    // Extract from requestBody
    if let Some(body) = operation.get("requestBody") {
        if let Some(content) = body.get("content") {
            if let Some(json) = content.get("application/json") {
                if let Some(schema) = json.get("schema") {
                    extract_schema_fields(schema, &mut fields);
                }
            }
        }
    }

    fields
}

fn extract_response_fields(operation: &serde_json::Value) -> Vec<FieldSpec> {
    let mut fields = Vec::new();

    if let Some(responses) = operation.get("responses").and_then(|r| r.as_object()) {
        // Check 200/201 responses
        for code in &["200", "201"] {
            if let Some(response) = responses.get(*code) {
                if let Some(content) = response.get("content") {
                    if let Some(json) = content.get("application/json") {
                        if let Some(schema) = json.get("schema") {
                            extract_schema_fields(schema, &mut fields);
                        }
                    }
                }
            }
        }
    }

    fields
}

fn extract_schema_fields(schema: &serde_json::Value, fields: &mut Vec<FieldSpec>) {
    let required_set: Vec<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        for (name, prop) in properties {
            let field_type = prop
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("object")
                .to_string();
            let nullable = prop
                .get("nullable")
                .and_then(|n| n.as_bool())
                .unwrap_or(false);

            fields.push(FieldSpec {
                name: name.clone(),
                field_type,
                required: required_set.contains(name),
                nullable,
            });
        }
    }
}
