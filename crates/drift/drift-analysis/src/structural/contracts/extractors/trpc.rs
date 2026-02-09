//! tRPC router extractor.

use super::EndpointExtractor;
use crate::structural::contracts::types::*;

pub struct TrpcExtractor;

impl EndpointExtractor for TrpcExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // .query('procedureName', ...) or .mutation('procedureName', ...)
            for (method_str, method) in &[(".query(", "QUERY"), (".mutation(", "MUTATION"), (".subscription(", "SUBSCRIPTION")] {
                if let Some(pos) = trimmed.find(method_str) {
                    if let Some(name) = super::express::extract_string_arg(trimmed, pos + method_str.len()) {
                        endpoints.push(Endpoint {
                            method: method.to_string(),
                            path: name,
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

    fn framework(&self) -> &str { "trpc" }
    fn matches(&self, content: &str) -> bool {
        content.contains("@trpc/server") || content.contains("createTRPCRouter")
            || content.contains("publicProcedure")
    }
}
