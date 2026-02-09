//! Language detection from file extension.

use serde::{Deserialize, Serialize};

/// Supported programming languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    TypeScript,
    JavaScript,
    Python,
    Java,
    CSharp,
    Go,
    Rust,
    Ruby,
    Php,
    Kotlin,
}

impl Language {
    /// Detect language from a file extension string.
    pub fn from_extension(ext: Option<&str>) -> Option<Language> {
        match ext? {
            "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
            "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
            "py" | "pyi" => Some(Language::Python),
            "java" => Some(Language::Java),
            "cs" => Some(Language::CSharp),
            "go" => Some(Language::Go),
            "rs" => Some(Language::Rust),
            "rb" | "rake" | "gemspec" => Some(Language::Ruby),
            "php" => Some(Language::Php),
            "kt" | "kts" => Some(Language::Kotlin),
            _ => None,
        }
    }

    /// Returns all file extensions associated with this language.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::TypeScript => &["ts", "tsx", "mts", "cts"],
            Language::JavaScript => &["js", "jsx", "mjs", "cjs"],
            Language::Python => &["py", "pyi"],
            Language::Java => &["java"],
            Language::CSharp => &["cs"],
            Language::Go => &["go"],
            Language::Rust => &["rs"],
            Language::Ruby => &["rb", "rake", "gemspec"],
            Language::Php => &["php"],
            Language::Kotlin => &["kt", "kts"],
        }
    }

    /// Returns the display name of the language.
    pub fn name(&self) -> &'static str {
        match self {
            Language::TypeScript => "TypeScript",
            Language::JavaScript => "JavaScript",
            Language::Python => "Python",
            Language::Java => "Java",
            Language::CSharp => "C#",
            Language::Go => "Go",
            Language::Rust => "Rust",
            Language::Ruby => "Ruby",
            Language::Php => "PHP",
            Language::Kotlin => "Kotlin",
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
