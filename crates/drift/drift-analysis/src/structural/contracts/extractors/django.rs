//! Django endpoint extractor (urlpatterns).

use super::EndpointExtractor;
use crate::structural::contracts::types::*;

pub struct DjangoExtractor;

impl EndpointExtractor for DjangoExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // path('api/users/', views.user_list, name='user-list')
            // re_path(r'^api/users/$', views.user_list)
            for prefix in &["path(", "re_path("] {
                if let Some(pos) = trimmed.find(prefix) {
                    if let Some(path) = extract_django_path(trimmed, pos + prefix.len()) {
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

            // @api_view(['GET', 'POST']) decorator
            if trimmed.contains("@api_view") {
                // The next function definition is the endpoint
                // We'll capture the decorator line for now
            }
        }
        endpoints
    }

    fn framework(&self) -> &str { "django" }
    fn matches(&self, content: &str) -> bool {
        content.contains("urlpatterns") || content.contains("@api_view") || content.contains("django")
    }
}

fn extract_django_path(line: &str, start: usize) -> Option<String> {
    let rest = &line[start..];
    let trimmed = rest.trim_start();
    // Handle both 'path' and "path" and r'regex'
    let trimmed = trimmed.trim_start_matches('r');
    let quote = trimmed.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let after_quote = &trimmed[1..];
    let end = after_quote.find(quote)?;
    Some(after_quote[..end].to_string())
}
