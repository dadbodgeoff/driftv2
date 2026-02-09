//! Taint sink extraction — feeds Phase 4 taint analysis.
//!
//! Identifies functions/methods that are security-sensitive sinks
//! (SQL execution, command execution, file I/O, etc.)

use serde::{Deserialize, Serialize};

use crate::scanner::language_detect::Language;

/// A taint sink definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintSink {
    pub name: String,
    pub receiver: Option<String>,
    pub category: SinkCategory,
    pub language: Language,
    pub tainted_params: Vec<usize>,
    pub severity: SinkSeverity,
}

/// Categories of taint sinks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SinkCategory {
    SqlExecution,
    CommandExecution,
    FileWrite,
    FileRead,
    NetworkRequest,
    HtmlRendering,
    Deserialization,
    Logging,
    Redirect,
    Eval,
}

/// Severity of a taint sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SinkSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Extract taint sink definitions for a given language.
pub fn extract_sinks(language: Language) -> Vec<TaintSink> {
    match language {
        Language::TypeScript | Language::JavaScript => typescript_sinks(),
        Language::Python => python_sinks(),
        Language::Java => java_sinks(),
        Language::CSharp => csharp_sinks(),
        Language::Go => go_sinks(),
        Language::Ruby => ruby_sinks(),
        Language::Php => php_sinks(),
        Language::Rust => rust_sinks(),
        Language::Kotlin => kotlin_sinks(),
    }
}

fn typescript_sinks() -> Vec<TaintSink> {
    vec![
        TaintSink { name: "eval".into(), receiver: None, category: SinkCategory::Eval, language: Language::JavaScript, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "exec".into(), receiver: None, category: SinkCategory::CommandExecution, language: Language::JavaScript, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "query".into(), receiver: Some("connection".into()), category: SinkCategory::SqlExecution, language: Language::JavaScript, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "innerHTML".into(), receiver: None, category: SinkCategory::HtmlRendering, language: Language::JavaScript, tainted_params: vec![0], severity: SinkSeverity::High },
        TaintSink { name: "writeFile".into(), receiver: Some("fs".into()), category: SinkCategory::FileWrite, language: Language::JavaScript, tainted_params: vec![1], severity: SinkSeverity::High },
        TaintSink { name: "redirect".into(), receiver: Some("res".into()), category: SinkCategory::Redirect, language: Language::JavaScript, tainted_params: vec![0], severity: SinkSeverity::Medium },
    ]
}

fn python_sinks() -> Vec<TaintSink> {
    vec![
        TaintSink { name: "eval".into(), receiver: None, category: SinkCategory::Eval, language: Language::Python, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "exec".into(), receiver: None, category: SinkCategory::Eval, language: Language::Python, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "system".into(), receiver: Some("os".into()), category: SinkCategory::CommandExecution, language: Language::Python, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "execute".into(), receiver: Some("cursor".into()), category: SinkCategory::SqlExecution, language: Language::Python, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "open".into(), receiver: None, category: SinkCategory::FileRead, language: Language::Python, tainted_params: vec![0], severity: SinkSeverity::High },
        TaintSink { name: "loads".into(), receiver: Some("pickle".into()), category: SinkCategory::Deserialization, language: Language::Python, tainted_params: vec![0], severity: SinkSeverity::Critical },
    ]
}

fn java_sinks() -> Vec<TaintSink> {
    vec![
        TaintSink { name: "executeQuery".into(), receiver: Some("Statement".into()), category: SinkCategory::SqlExecution, language: Language::Java, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "exec".into(), receiver: Some("Runtime".into()), category: SinkCategory::CommandExecution, language: Language::Java, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "readObject".into(), receiver: Some("ObjectInputStream".into()), category: SinkCategory::Deserialization, language: Language::Java, tainted_params: vec![], severity: SinkSeverity::Critical },
    ]
}

fn csharp_sinks() -> Vec<TaintSink> {
    vec![
        TaintSink { name: "ExecuteNonQuery".into(), receiver: Some("SqlCommand".into()), category: SinkCategory::SqlExecution, language: Language::CSharp, tainted_params: vec![], severity: SinkSeverity::Critical },
        TaintSink { name: "Start".into(), receiver: Some("Process".into()), category: SinkCategory::CommandExecution, language: Language::CSharp, tainted_params: vec![0], severity: SinkSeverity::Critical },
    ]
}

fn go_sinks() -> Vec<TaintSink> {
    vec![
        TaintSink { name: "Exec".into(), receiver: Some("db".into()), category: SinkCategory::SqlExecution, language: Language::Go, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "Command".into(), receiver: Some("exec".into()), category: SinkCategory::CommandExecution, language: Language::Go, tainted_params: vec![0], severity: SinkSeverity::Critical },
    ]
}

fn ruby_sinks() -> Vec<TaintSink> {
    vec![
        TaintSink { name: "eval".into(), receiver: None, category: SinkCategory::Eval, language: Language::Ruby, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "system".into(), receiver: None, category: SinkCategory::CommandExecution, language: Language::Ruby, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "execute".into(), receiver: Some("ActiveRecord".into()), category: SinkCategory::SqlExecution, language: Language::Ruby, tainted_params: vec![0], severity: SinkSeverity::Critical },
    ]
}

fn php_sinks() -> Vec<TaintSink> {
    vec![
        TaintSink { name: "eval".into(), receiver: None, category: SinkCategory::Eval, language: Language::Php, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "exec".into(), receiver: None, category: SinkCategory::CommandExecution, language: Language::Php, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "query".into(), receiver: Some("PDO".into()), category: SinkCategory::SqlExecution, language: Language::Php, tainted_params: vec![0], severity: SinkSeverity::Critical },
    ]
}

fn rust_sinks() -> Vec<TaintSink> {
    vec![
        TaintSink { name: "execute".into(), receiver: Some("Connection".into()), category: SinkCategory::SqlExecution, language: Language::Rust, tainted_params: vec![0], severity: SinkSeverity::Critical },
        TaintSink { name: "Command".into(), receiver: Some("std::process".into()), category: SinkCategory::CommandExecution, language: Language::Rust, tainted_params: vec![0], severity: SinkSeverity::Critical },
    ]
}

fn kotlin_sinks() -> Vec<TaintSink> {
    vec![
        TaintSink { name: "executeQuery".into(), receiver: Some("Statement".into()), category: SinkCategory::SqlExecution, language: Language::Kotlin, tainted_params: vec![0], severity: SinkSeverity::Critical },
    ]
}
