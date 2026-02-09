//! Actix-web (Rust) endpoint extractor.

use super::EndpointExtractor;
use super::express::extract_string_arg;
use crate::structural::contracts::types::*;

pub struct ActixExtractor;

impl EndpointExtractor for ActixExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();
        let attrs = [
            ("#[get(", "GET"), ("#[post(", "POST"), ("#[put(", "PUT"),
            ("#[delete(", "DELETE"), ("#[patch(", "PATCH"),
        ];

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            for (attr, method) in &attrs {
                if let Some(pos) = trimmed.find(attr) {
                    if let Some(path) = extract_string_arg(trimmed, pos + attr.len()) {
                        endpoints.push(Endpoint {
                            method: method.to_string(),
                            path,
                            request_fields: vec![],
                            response_fields: vec![],
                            file: file_path.to_string(),
                            line: (line_num + 1) as u32,
                        });
                    }
                }
            }

            // web::resource("/users").route(web::get().to(handler))
            if trimmed.contains("web::resource(") {
                if let Some(pos) = trimmed.find("web::resource(") {
                    if let Some(path) = extract_string_arg(trimmed, pos + 14) {
                        endpoints.push(Endpoint {
                            method: "ANY".to_string(),
                            path,
                            request_fields: vec![],
                            response_fields: vec![],
                            file: file_path.to_string(),
                            line: (line_num + 1) as u32,
                        });
                    }
                }
            }
        }
        endpoints
    }

    fn framework(&self) -> &str { "actix" }
    fn matches(&self, content: &str) -> bool {
        content.contains("actix_web") || content.contains("actix-web")
            || content.contains("#[get(") || content.contains("web::resource")
    }
}
