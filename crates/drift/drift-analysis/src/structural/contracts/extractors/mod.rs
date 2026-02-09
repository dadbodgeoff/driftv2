//! Backend and frontend endpoint extractors.

pub mod express;
pub mod fastify;
pub mod nestjs;
pub mod django;
pub mod flask;
pub mod spring;
pub mod aspnet;
pub mod rails;
pub mod laravel;
pub mod gin;
pub mod actix;
pub mod nextjs;
pub mod trpc;
pub mod frontend;

use super::types::Endpoint;

/// Trait for extracting API endpoints from source code.
pub trait EndpointExtractor: Send + Sync {
    /// Extract endpoints from source code content.
    fn extract(&self, content: &str, file_path: &str) -> Vec<Endpoint>;
    /// Framework name.
    fn framework(&self) -> &str;
    /// Check if this extractor applies to the given file content.
    fn matches(&self, content: &str) -> bool;
}

/// Registry of all endpoint extractors.
pub struct ExtractorRegistry {
    extractors: Vec<Box<dyn EndpointExtractor>>,
}

impl ExtractorRegistry {
    /// Create a registry with all built-in extractors.
    pub fn new() -> Self {
        Self {
            extractors: vec![
                Box::new(express::ExpressExtractor),
                Box::new(fastify::FastifyExtractor),
                Box::new(nestjs::NestJsExtractor),
                Box::new(django::DjangoExtractor),
                Box::new(flask::FlaskExtractor),
                Box::new(spring::SpringExtractor),
                Box::new(aspnet::AspNetExtractor),
                Box::new(rails::RailsExtractor),
                Box::new(laravel::LaravelExtractor),
                Box::new(gin::GinExtractor),
                Box::new(actix::ActixExtractor),
                Box::new(nextjs::NextJsExtractor),
                Box::new(trpc::TrpcExtractor),
                Box::new(frontend::FrontendExtractor),
            ],
        }
    }

    /// Extract endpoints from a file using all matching extractors.
    pub fn extract_all(&self, content: &str, file_path: &str) -> Vec<(String, Vec<Endpoint>)> {
        self.extractors
            .iter()
            .filter(|e| e.matches(content))
            .map(|e| (e.framework().to_string(), e.extract(content, file_path)))
            .filter(|(_, eps)| !eps.is_empty())
            .collect()
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
