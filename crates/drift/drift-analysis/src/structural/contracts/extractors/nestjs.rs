//! NestJS endpoint extractor (decorator-based).

use super::EndpointExtractor;
use super::express::extract_string_arg;
use crate::structural::contracts::types::*;

pub struct NestJsExtractor;

impl EndpointExtractor for NestJsExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();
        let decorators = [
            ("@Get(", "GET"), ("@Post(", "POST"), ("@Put(", "PUT"),
            ("@Delete(", "DELETE"), ("@Patch(", "PATCH"),
        ];

        // Extract controller base path
        let base_path = extract_controller_path(content).unwrap_or_default();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            for (decorator, method) in &decorators {
                if let Some(pos) = trimmed.find(decorator) {
                    let path = extract_string_arg(trimmed, pos + decorator.len())
                        .unwrap_or_default();
                    let full_path = if base_path.is_empty() {
                        path
                    } else {
                        format!("{}/{}", base_path.trim_end_matches('/'), path.trim_start_matches('/'))
                    };
                    endpoints.push(Endpoint {
                        method: method.to_string(),
                        path: full_path,
                        request_fields: vec![],
                        response_fields: vec![],
                        file: file_path.to_string(),
                        line: (line_num + 1) as u32,
                    });
                }
            }
        }
        endpoints
    }

    fn framework(&self) -> &str { "nestjs" }
    fn matches(&self, content: &str) -> bool {
        content.contains("@Controller") || content.contains("@nestjs/common")
    }
}

fn extract_controller_path(content: &str) -> Option<String> {
    let marker = "@Controller(";
    let pos = content.find(marker)?;
    extract_string_arg(content, pos + marker.len())
}
