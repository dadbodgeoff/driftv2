//! AsyncAPI 2.x/3.0 schema parser.

use super::SchemaParser;
use crate::structural::contracts::types::*;

/// Parses AsyncAPI specifications (JSON/YAML).
pub struct AsyncApiParser;

impl SchemaParser for AsyncApiParser {
    fn parse(&self, content: &str, file_path: &str) -> Vec<Contract> {
        let value: Option<serde_json::Value> = serde_json::from_str(content)
            .ok()
            .or_else(|| serde_yaml::from_str(content).ok());

        let value = match value {
            Some(v) => v,
            None => return vec![],
        };

        let mut endpoints = Vec::new();

        // AsyncAPI uses "channels" instead of "paths"
        if let Some(channels) = value.get("channels").and_then(|c| c.as_object()) {
            for (channel_name, channel) in channels {
                // publish/subscribe operations
                for op_type in &["publish", "subscribe"] {
                    if let Some(operation) = channel.get(*op_type) {
                        let fields = extract_message_fields(operation);
                        endpoints.push(Endpoint {
                            method: op_type.to_uppercase(),
                            path: channel_name.clone(),
                            request_fields: fields.clone(),
                            response_fields: vec![],
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
            id: format!("asyncapi:{}", file_path),
            paradigm: Paradigm::AsyncApi,
            endpoints,
            source_file: file_path.to_string(),
            framework: "asyncapi".to_string(),
            confidence: 0.85,
        }]
    }

    fn extensions(&self) -> &[&str] {
        &["yaml", "yml", "json"]
    }

    fn schema_type(&self) -> &str {
        "asyncapi"
    }
}

fn extract_message_fields(operation: &serde_json::Value) -> Vec<FieldSpec> {
    let mut fields = Vec::new();

    if let Some(message) = operation.get("message") {
        if let Some(payload) = message.get("payload") {
            if let Some(properties) = payload.get("properties").and_then(|p| p.as_object()) {
                for (name, prop) in properties {
                    let field_type = prop
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("string")
                        .to_string();
                    fields.push(FieldSpec {
                        name: name.clone(),
                        field_type,
                        required: false,
                        nullable: false,
                    });
                }
            }
        }
    }

    fields
}
