//! Express.js endpoint extractor.

use super::EndpointExtractor;
use crate::structural::contracts::types::*;

pub struct ExpressExtractor;

impl EndpointExtractor for ExpressExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();
        let methods = ["get", "post", "put", "delete", "patch"];

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            for method in &methods {
                // app.get('/path', ...) or router.get('/path', ...)
                let patterns = [
                    format!("app.{}(", method),
                    format!("router.{}(", method),
                ];
                for pattern in &patterns {
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
            }
        }
        endpoints
    }

    fn framework(&self) -> &str { "express" }

    fn matches(&self, content: &str) -> bool {
        content.contains("express") || content.contains("app.get(") || content.contains("router.get(")
    }
}

/// Extract a string argument from a position (handles both ' and " quotes).
pub(crate) fn extract_string_arg(line: &str, start: usize) -> Option<String> {
    let rest = &line[start..];
    let trimmed = rest.trim_start();
    let quote = trimmed.chars().next()?;
    if quote != '\'' && quote != '"' && quote != '`' {
        return None;
    }
    let after_quote = &trimmed[1..];
    let end = after_quote.find(quote)?;
    Some(after_quote[..end].to_string())
}
