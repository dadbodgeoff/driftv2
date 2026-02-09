//! ASP.NET endpoint extractor.

use super::EndpointExtractor;
use super::express::extract_string_arg;
use crate::structural::contracts::types::*;

pub struct AspNetExtractor;

impl EndpointExtractor for AspNetExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();
        let attrs = [
            ("[HttpGet(", "GET"), ("[HttpPost(", "POST"),
            ("[HttpPut(", "PUT"), ("[HttpDelete(", "DELETE"),
            ("[HttpPatch(", "PATCH"),
        ];

        let base_path = extract_route_attr(content).unwrap_or_default();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            for (attr, method) in &attrs {
                if let Some(pos) = trimmed.find(attr) {
                    let path = extract_string_arg(trimmed, pos + attr.len()).unwrap_or_default();
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

    fn framework(&self) -> &str { "aspnet" }
    fn matches(&self, content: &str) -> bool {
        content.contains("[ApiController]") || content.contains("[HttpGet")
            || content.contains("Microsoft.AspNetCore")
    }
}

fn extract_route_attr(content: &str) -> Option<String> {
    let marker = "[Route(";
    let pos = content.find(marker)?;
    extract_string_arg(content, pos + marker.len())
}
