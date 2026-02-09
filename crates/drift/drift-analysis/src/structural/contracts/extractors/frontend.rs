//! Frontend/consumer library extractors (fetch, axios, SWR, TanStack Query, Apollo, urql).

use super::EndpointExtractor;
use super::express::extract_string_arg;
use crate::structural::contracts::types::*;

pub struct FrontendExtractor;

impl EndpointExtractor for FrontendExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // fetch('/api/users') or fetch("/api/users")
            if let Some(pos) = trimmed.find("fetch(") {
                if let Some(path) = extract_string_arg(trimmed, pos + 6) {
                    if path.starts_with('/') || path.starts_with("http") {
                        let method = infer_method(trimmed);
                        endpoints.push(Endpoint {
                            method,
                            path,
                            request_fields: vec![],
                            response_fields: vec![],
                            file: file_path.to_string(),
                            line: (line_num + 1) as u32,
                        });
                    }
                }
            }

            // axios.get('/api/users') or axios.post('/api/users')
            for method in &["get", "post", "put", "delete", "patch"] {
                let pattern = format!("axios.{}(", method);
                if let Some(pos) = trimmed.find(pattern.as_str()) {
                    if let Some(path) = extract_string_arg(trimmed, pos + pattern.len()) {
                        endpoints.push(Endpoint {
                            method: method.to_uppercase(),
                            path,
                            request_fields: vec![],
                            response_fields: vec![],
                            file: file_path.to_string(),
                            line: (line_num + 1) as u32,
                        });
                    }
                }
            }

            // useSWR('/api/users', fetcher) or useQuery(['/api/users'])
            for hook in &["useSWR(", "useQuery("] {
                if let Some(pos) = trimmed.find(hook) {
                    if let Some(path) = extract_string_arg(trimmed, pos + hook.len()) {
                        if path.starts_with('/') || path.starts_with("http") {
                            endpoints.push(Endpoint {
                                method: "GET".to_string(),
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
        }
        endpoints
    }

    fn framework(&self) -> &str { "frontend" }
    fn matches(&self, content: &str) -> bool {
        content.contains("fetch(") || content.contains("axios")
            || content.contains("useSWR") || content.contains("useQuery")
            || content.contains("useMutation")
    }
}

fn infer_method(line: &str) -> String {
    if line.contains("method:") {
        let lower = line.to_lowercase();
        if lower.contains("'post'") || lower.contains("\"post\"") {
            return "POST".to_string();
        }
        if lower.contains("'put'") || lower.contains("\"put\"") {
            return "PUT".to_string();
        }
        if lower.contains("'delete'") || lower.contains("\"delete\"") {
            return "DELETE".to_string();
        }
        if lower.contains("'patch'") || lower.contains("\"patch\"") {
            return "PATCH".to_string();
        }
    }
    "GET".to_string()
}
