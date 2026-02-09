//! ParserManager — routes files to the correct language parser.

use std::path::Path;

use drift_core::errors::ParseError;

use super::cache::ParseCache;
use super::languages::csharp::CSharpParser;
use super::languages::go::GoParser;
use super::languages::java::JavaParser;
use super::languages::javascript::JavaScriptParser;
use super::languages::kotlin::KotlinParser;
use super::languages::php::PhpParser;
use super::languages::python::PythonParser;
use super::languages::ruby::RubyParser;
use super::languages::rust_lang::RustParser;
use super::languages::typescript::TypeScriptParser;
use super::traits::LanguageParser;
use super::types::ParseResult;
use crate::scanner::hasher::hash_content;
use crate::scanner::language_detect::Language;

/// Manages all language parsers and the parse cache.
pub struct ParserManager {
    cache: ParseCache,
    typescript: TypeScriptParser,
    javascript: JavaScriptParser,
    python: PythonParser,
    java: JavaParser,
    csharp: CSharpParser,
    go: GoParser,
    rust_parser: RustParser,
    ruby: RubyParser,
    php: PhpParser,
    kotlin: KotlinParser,
}

impl ParserManager {
    /// Create a new ParserManager with default cache capacity.
    pub fn new() -> Self {
        Self {
            cache: ParseCache::default(),
            typescript: TypeScriptParser::new(),
            javascript: JavaScriptParser::new(),
            python: PythonParser::new(),
            java: JavaParser::new(),
            csharp: CSharpParser::new(),
            go: GoParser::new(),
            rust_parser: RustParser::new(),
            ruby: RubyParser::new(),
            php: PhpParser::new(),
            kotlin: KotlinParser::new(),
        }
    }

    /// Create a new ParserManager with a specific cache capacity.
    pub fn with_cache_capacity(capacity: u64) -> Self {
        Self {
            cache: ParseCache::new(capacity),
            ..Self::new()
        }
    }

    /// Get the parser for a given language.
    fn parser_for(&self, lang: Language) -> &dyn LanguageParser {
        match lang {
            Language::TypeScript => &self.typescript,
            Language::JavaScript => &self.javascript,
            Language::Python => &self.python,
            Language::Java => &self.java,
            Language::CSharp => &self.csharp,
            Language::Go => &self.go,
            Language::Rust => &self.rust_parser,
            Language::Ruby => &self.ruby,
            Language::Php => &self.php,
            Language::Kotlin => &self.kotlin,
        }
    }

    /// Detect language from file extension.
    pub fn detect_language(&self, path: &Path) -> Option<Language> {
        Language::from_extension(path.extension().and_then(|e| e.to_str()))
    }

    /// Parse a file, using the cache if available.
    pub fn parse(&self, source: &[u8], path: &Path) -> Result<ParseResult, ParseError> {
        let lang = self.detect_language(path).ok_or_else(|| {
            ParseError::UnsupportedLanguage {
                extension: path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            }
        })?;

        let content_hash = hash_content(source);

        // Check cache
        if let Some(cached) = self.cache.get(content_hash) {
            return Ok(cached);
        }

        // Parse
        let parser = self.parser_for(lang);
        let result = parser.parse(source, path)?;

        // Cache the result
        self.cache.insert(content_hash, result.clone());

        Ok(result)
    }

    /// Parse a file with a known language (skips detection).
    pub fn parse_with_language(
        &self,
        source: &[u8],
        path: &Path,
        lang: Language,
    ) -> Result<ParseResult, ParseError> {
        let content_hash = hash_content(source);

        if let Some(cached) = self.cache.get(content_hash) {
            return Ok(cached);
        }

        let parser = self.parser_for(lang);
        let result = parser.parse(source, path)?;
        self.cache.insert(content_hash, result.clone());
        Ok(result)
    }

    /// Get the number of cached parse results.
    pub fn cache_entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Invalidate a cache entry by content hash.
    pub fn invalidate_cache(&self, content_hash: u64) {
        self.cache.invalidate(content_hash);
    }
}

impl Default for ParserManager {
    fn default() -> Self {
        Self::new()
    }
}
