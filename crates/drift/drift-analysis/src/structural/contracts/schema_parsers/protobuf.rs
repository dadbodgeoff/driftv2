//! Protobuf schema parser (gRPC).

use super::SchemaParser;
use crate::structural::contracts::types::*;

/// Parses Protocol Buffer definitions for gRPC services.
pub struct ProtobufParser;

impl SchemaParser for ProtobufParser {
    fn parse(&self, content: &str, file_path: &str) -> Vec<Contract> {
        let mut endpoints = Vec::new();

        // Find service definitions: service ServiceName { rpc Method(Request) returns (Response); }
        let mut pos = 0;
        while let Some(svc_start) = content[pos..].find("service ") {
            let abs_start = pos + svc_start;
            let rest = &content[abs_start + 8..];

            if let Some(brace) = rest.find('{') {
                let block_start = abs_start + 8 + brace + 1;
                if let Some(block_end) = find_matching_brace(content, block_start) {
                    let block = &content[block_start..block_end];

                    for line in block.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("rpc ") {
                            if let Some(ep) = parse_rpc_line(trimmed, file_path) {
                                endpoints.push(ep);
                            }
                        }
                    }

                    pos = block_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if endpoints.is_empty() {
            return vec![];
        }

        vec![Contract {
            id: format!("grpc:{}", file_path),
            paradigm: Paradigm::Grpc,
            endpoints,
            source_file: file_path.to_string(),
            framework: "grpc".to_string(),
            confidence: 0.90,
        }]
    }

    fn extensions(&self) -> &[&str] {
        &["proto"]
    }

    fn schema_type(&self) -> &str {
        "protobuf"
    }
}

fn find_matching_brace(content: &str, start: usize) -> Option<usize> {
    let mut depth = 1;
    for (i, ch) in content[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_rpc_line(line: &str, file_path: &str) -> Option<Endpoint> {
    // rpc MethodName(RequestType) returns (ResponseType);
    let rest = line.strip_prefix("rpc ")?.trim();
    let paren_start = rest.find('(')?;
    let method_name = rest[..paren_start].trim();
    let after_name = &rest[paren_start + 1..];
    let paren_end = after_name.find(')')?;
    let request_type = after_name[..paren_end].trim();

    let returns_start = after_name.find("returns")? + 7;
    let resp_paren_start = after_name[returns_start..].find('(')? + returns_start + 1;
    let resp_paren_end = after_name[resp_paren_start..].find(')')? + resp_paren_start;
    let response_type = after_name[resp_paren_start..resp_paren_end].trim();

    Some(Endpoint {
        method: "RPC".to_string(),
        path: method_name.to_string(),
        request_fields: vec![FieldSpec {
            name: "request".to_string(),
            field_type: request_type.to_string(),
            required: true,
            nullable: false,
        }],
        response_fields: vec![FieldSpec {
            name: "response".to_string(),
            field_type: response_type.to_string(),
            required: true,
            nullable: false,
        }],
        file: file_path.to_string(),
        line: 0,
    })
}
