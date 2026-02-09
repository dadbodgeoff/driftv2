//! Rails endpoint extractor (routes.rb).

use super::EndpointExtractor;
use crate::structural::contracts::types::*;

pub struct RailsExtractor;

impl EndpointExtractor for RailsExtractor {
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint> {
        let mut endpoints = Vec::new();
        let methods = ["get", "post", "put", "delete", "patch"];

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            for method in &methods {
                // get '/users', to: 'users#index'
                let pattern = format!("{} '", method);
                let pattern2 = format!("{} \"", method);
                for pat in &[&pattern, &pattern2] {
                    if trimmed.starts_with(pat.as_str()) {
                        let quote = pat.chars().last().unwrap();
                        let rest = &trimmed[pat.len()..];
                        if let Some(end) = rest.find(quote) {
                            endpoints.push(Endpoint {
                                method: method.to_uppercase(),
                                path: rest[..end].to_string(),
                                request_fields: vec![],
                                response_fields: vec![],
                                file: file_path.to_string(),
                                line: (line_num + 1) as u32,
                            });
                        }
                    }
                }
            }

            // resources :users
            if trimmed.starts_with("resources :") || trimmed.starts_with("resources(") {
                let resource = trimmed
                    .trim_start_matches("resources :")
                    .trim_start_matches("resources(:")
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("");
                if !resource.is_empty() {
                    for (method, action) in &[("GET", "index"), ("GET", "show"), ("POST", "create"),
                                               ("PUT", "update"), ("DELETE", "destroy")] {
                        endpoints.push(Endpoint {
                            method: method.to_string(),
                            path: format!("/{}", resource),
                            request_fields: vec![],
                            response_fields: vec![],
                            file: file_path.to_string(),
                            line: (line_num + 1) as u32,
                        });
                        let _ = action; // Used for documentation
                    }
                }
            }
        }
        endpoints
    }

    fn framework(&self) -> &str { "rails" }
    fn matches(&self, content: &str) -> bool {
        content.contains("Rails.application.routes") || content.contains("resources :")
            || content.contains("ActionController")
    }
}
