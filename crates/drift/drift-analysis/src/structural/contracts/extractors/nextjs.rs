//! Next.js API route extractor.

use super::EndpointExtractor;
use crate::structural::contracts::types::*;

pub struct NextJsExtractor;

impl EndpointExtractor for NextJsExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();

        // Next.js App Router: export async function GET/POST/PUT/DELETE/PATCH
        let methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            for method in &methods {
                let patterns = [
                    format!("export async function {}", method),
                    format!("export function {}", method),
                    format!("export const {} =", method),
                ];
                for pattern in &patterns {
                    if trimmed.starts_with(pattern.as_str()) {
                        let path = file_path_to_api_route(file_path);
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

        // Pages Router: export default function handler
        if content.contains("export default") && file_path.contains("pages/api/") {
            let path = file_path_to_api_route(file_path);
            if endpoints.is_empty() {
                endpoints.push(Endpoint {
                    method: "ANY".to_string(),
                    path,
                    request_fields: vec![],
                    response_fields: vec![],
                    file: file_path.to_string(),
                    line: 1,
                });
            }
        }

        endpoints
    }

    fn framework(&self) -> &str { "nextjs" }
    fn matches(&self, content: &str) -> bool {
        content.contains("NextRequest") || content.contains("NextResponse")
            || content.contains("NextApiRequest")
    }
}

fn file_path_to_api_route(file_path: &str) -> String {
    let normalized = file_path.replace('\\', "/");
    // app/api/users/route.ts → /api/users
    if let Some(pos) = normalized.find("app/api/") {
        let route = &normalized[pos + 4..]; // skip "app/"
        let route = route.trim_end_matches("/route.ts")
            .trim_end_matches("/route.js")
            .trim_end_matches("/route.tsx");
        return format!("/{}", route.trim_start_matches('/'));
    }
    // pages/api/users.ts → /api/users
    if let Some(pos) = normalized.find("pages/api/") {
        let route = &normalized[pos + 6..]; // skip "pages/"
        let route = route.trim_end_matches(".ts")
            .trim_end_matches(".js")
            .trim_end_matches(".tsx")
            .trim_end_matches("/index");
        return format!("/{}", route.trim_start_matches('/'));
    }
    normalized
}
