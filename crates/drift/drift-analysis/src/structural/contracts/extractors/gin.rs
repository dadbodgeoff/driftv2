//! Gin (Go) endpoint extractor.

use super::EndpointExtractor;
use super::express::extract_string_arg;
use crate::structural::contracts::types::*;

pub struct GinExtractor;

impl EndpointExtractor for GinExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();
        let methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            for method in &methods {
                // r.GET("/users", handler) or group.GET("/users", handler)
                let pattern = format!(".{}(", method);
                if let Some(pos) = trimmed.find(pattern.as_str()) {
                    if let Some(path) = extract_string_arg(trimmed, pos + pattern.len()) {
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
        }
        endpoints
    }

    fn framework(&self) -> &str { "gin" }
    fn matches(&self, content: &str) -> bool {
        content.contains("gin.Default()") || content.contains("gin.New()")
            || content.contains("github.com/gin-gonic/gin")
    }
}
