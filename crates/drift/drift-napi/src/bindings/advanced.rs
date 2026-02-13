//! NAPI bindings for Phase 7 advanced systems.
//!
//! Exposes: drift_simulate(), drift_decisions(), drift_context(), drift_generate_spec()

use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde_json;

/// Simulate task approaches with Monte Carlo confidence intervals.
#[napi]
pub async fn drift_simulate(
    task_category: String,
    task_description: String,
    context_json: String,
) -> Result<String> {
    use drift_analysis::advanced::simulation::types::*;
    use drift_analysis::advanced::simulation::strategies::StrategyRecommender;

    // PH2-08: Parse affected_files alongside context from JSON input
    #[derive(serde::Deserialize, Default)]
    struct SimulationInput {
        #[serde(flatten)]
        context: SimulationContext,
        #[serde(default)]
        affected_files: Vec<String>,
    }

    let input: SimulationInput = serde_json::from_str(&context_json)
        .unwrap_or_default();
    let context = input.context;
    let affected_files = input.affected_files;

    let category = match task_category.as_str() {
        "add_feature" => TaskCategory::AddFeature,
        "fix_bug" => TaskCategory::FixBug,
        "refactor" => TaskCategory::Refactor,
        "migrate_framework" => TaskCategory::MigrateFramework,
        "add_test" => TaskCategory::AddTest,
        "security_fix" => TaskCategory::SecurityFix,
        "performance_optimization" => TaskCategory::PerformanceOptimization,
        "dependency_update" => TaskCategory::DependencyUpdate,
        "api_change" => TaskCategory::ApiChange,
        "database_migration" => TaskCategory::DatabaseMigration,
        "config_change" => TaskCategory::ConfigChange,
        "documentation" => TaskCategory::Documentation,
        "infrastructure" => TaskCategory::Infrastructure,
        _ => return Err(Error::from_reason(format!("Unknown task category: {}", task_category))),
    };

    let task = SimulationTask {
        category,
        description: task_description,
        affected_files,
        context,
    };

    let recommender = StrategyRecommender::new();
    let result = recommender.recommend(&task);

    serde_json::to_string(&result)
        .map_err(|e| Error::from_reason(format!("Serialization error: {}", e)))
}

/// Mine decisions from git history.
#[napi]
pub async fn drift_decisions(repo_path: String) -> Result<String> {
    use drift_analysis::advanced::decisions::git_analysis::GitAnalyzer;

    let analyzer = GitAnalyzer::new().with_max_commits(500);
    let decisions = analyzer.analyze(std::path::Path::new(&repo_path))
        .map_err(|e| Error::from_reason(format!("Git analysis error: {}", e)))?;

    serde_json::to_string(&decisions)
        .map_err(|e| Error::from_reason(format!("Serialization error: {}", e)))
}

/// Generate context for a given intent and depth.
///
/// When `data_json` is empty or `'{}'`, the engine automatically gathers
/// data from drift.db based on the intent.  Callers can still pass explicit
/// sections via `data_json` to override or supplement the gathered data.
#[napi]
pub async fn drift_context(
    intent: String,
    depth: String,
    data_json: String,
) -> Result<String> {
    use drift_context::generation::builder::*;
    use drift_context::generation::intent::ContextIntent;

    let intent = match intent.as_str() {
        "fix_bug" => ContextIntent::FixBug,
        "add_feature" => ContextIntent::AddFeature,
        "understand_code" | "understand" | "review_code" | "review" => ContextIntent::UnderstandCode,
        "security_audit" | "security" => ContextIntent::SecurityAudit,
        "generate_spec" | "spec" => ContextIntent::GenerateSpec,
        // Map analytical intents to the closest supported intent.
        // The Rust ContextIntent enum has 5 variants; these aliases let
        // callers use more descriptive names without breaking.
        "refactor" | "explain_pattern" | "documentation" => ContextIntent::UnderstandCode,
        "debug" | "trace_dependency" => ContextIntent::FixBug,
        "performance_audit" | "assess_risk" => ContextIntent::SecurityAudit,
        _ => return Err(Error::from_reason(format!(
            "Unknown intent: '{}'. Valid intents: fix_bug, add_feature, understand_code, security_audit, generate_spec",
            intent
        ))),
    };

    let depth = match depth.as_str() {
        "overview" => ContextDepth::Overview,
        "standard" => ContextDepth::Standard,
        "deep" => ContextDepth::Deep,
        _ => return Err(Error::from_reason(format!("Unknown depth: {}", depth))),
    };

    let explicit_sections: std::collections::HashMap<String, String> =
        serde_json::from_str(&data_json).unwrap_or_default();

    let mut data = AnalysisData::new();

    // Always gather data from drift.db.  This populates the standard sections
    // (overview, conventions, taint_analysis, etc.) from the analysis database.
    // Explicit sections from the caller override gathered ones if names collide,
    // and any extra caller-provided sections (e.g. violation_context) are added.
    gather_sections_from_db(&mut data, intent)?;

    // Merge explicit sections (override gathered ones if names collide).
    for (k, v) in explicit_sections {
        if !v.is_empty() {
            data.add_section(k, v);
        }
    }

    let mut engine = ContextEngine::new();
    let output = engine.generate(intent, depth, &data)
        .map_err(|e| Error::from_reason(format!("Context generation error: {}", e)))?;

    let result = serde_json::json!({
        "sections": output.sections.iter().map(|(n, c)| {
            serde_json::json!({"name": n, "content": c})
        }).collect::<Vec<_>>(),
        "token_count": output.token_count,
        "intent": output.intent.name(),
        "depth": output.depth.name(),
    });

    serde_json::to_string(&result)
        .map_err(|e| Error::from_reason(format!("Serialization error: {}", e)))
}

/// Gather context sections from drift.db based on the intent.
///
/// Each section corresponds to a weighted dimension in `IntentWeights`.
/// We query the relevant tables and format the results into human-readable
/// summaries that the `ContextEngine` can then weight and truncate.
fn gather_sections_from_db(
    data: &mut drift_context::generation::builder::AnalysisData,
    _intent: drift_context::generation::intent::ContextIntent,
) -> Result<()> {
    let rt = crate::runtime::get()?;

    // ── overview: file count, detection count, violation count ────────
    let file_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM file_metadata")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    let detection_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM detections")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    let violation_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM violations")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    data.add_section("overview", format!(
        "Project: {} files scanned, {} pattern detections, {} violations.",
        file_count, detection_count, violation_count,
    ));

    // ── conventions: top conventions by dominance ─────────────────────
    let conventions = rt.storage.with_reader(|conn| {
        drift_storage::queries::patterns::query_all_conventions(conn)
    }).unwrap_or_default();

    if !conventions.is_empty() {
        // Deduplicate by pattern_id — keep the first (highest dominance) entry
        // since the query is ORDER BY dominance_ratio DESC.
        let mut seen = std::collections::HashSet::new();
        let deduped: Vec<_> = conventions.iter()
            .filter(|c| seen.insert(c.pattern_id.clone()))
            .collect();

        let top: Vec<String> = deduped.iter().take(20).map(|c| {
            format!("- {} (category: {}, dominance: {:.0}%, scope: {})",
                c.pattern_id, c.category, c.dominance_ratio * 100.0, c.scope)
        }).collect();
        data.add_section("conventions", format!(
            "{} conventions discovered ({} unique). Top {}:\n{}",
            conventions.len(), deduped.len(), top.len(), top.join("\n"),
        ));
    }

    // ── error_handling: gap summary ──────────────────────────────────
    let error_gaps = rt.storage.with_reader(|conn| {
        conn.prepare_cached(
            "SELECT gap_type, severity, COUNT(*) as cnt FROM error_gaps GROUP BY gap_type, severity ORDER BY cnt DESC LIMIT 15"
        )
        .and_then(|mut s| {
            let rows = s.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
        })
        .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or_default();

    if !error_gaps.is_empty() {
        let lines: Vec<String> = error_gaps.iter().map(|(gap, sev, cnt)| {
            format!("- {} ({} severity): {} occurrences", gap, sev, cnt)
        }).collect();
        data.add_section("error_handling", format!(
            "Error handling gaps:\n{}", lines.join("\n"),
        ));
    }

    // ── taint_analysis: flow summary ─────────────────────────────────
    let taint_summary = rt.storage.with_reader(|conn| {
        conn.prepare_cached(
            "SELECT source_type, sink_type, COUNT(*) as cnt, SUM(CASE WHEN is_sanitized THEN 1 ELSE 0 END) as sanitized
             FROM taint_flows GROUP BY source_type, sink_type ORDER BY cnt DESC LIMIT 10"
        )
        .and_then(|mut s| {
            let rows = s.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
        })
        .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or_default();

    if !taint_summary.is_empty() {
        let lines: Vec<String> = taint_summary.iter().map(|(src, sink, cnt, san)| {
            format!("- {} → {}: {} flows ({} sanitized)", src, sink, cnt, san)
        }).collect();
        data.add_section("taint_analysis", format!(
            "Taint flows:\n{}", lines.join("\n"),
        ));
    }

    // ── call_graph: function and edge counts ─────────────────────────
    let func_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM functions")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    let edge_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM call_edges")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    if func_count > 0 {
        data.add_section("call_graph", format!(
            "Call graph: {} functions, {} edges.", func_count, edge_count,
        ));
    }

    // ── public_api: contract summary ─────────────────────────────────
    let contracts = rt.storage.with_reader(|conn| {
        conn.prepare_cached(
            "SELECT framework, paradigm, COUNT(*) as cnt FROM contracts GROUP BY framework, paradigm ORDER BY cnt DESC LIMIT 10"
        )
        .and_then(|mut s| {
            let rows = s.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
        })
        .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or_default();

    if !contracts.is_empty() {
        let lines: Vec<String> = contracts.iter().map(|(fw, paradigm, cnt)| {
            format!("- {} ({}): {} contracts", fw, paradigm, cnt)
        }).collect();
        data.add_section("public_api", format!(
            "API contracts:\n{}", lines.join("\n"),
        ));
    }

    // ── data_model: boundary and sensitive field summary ──────────────
    let boundary_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM boundaries")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    let sensitive_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM sensitive_fields")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    if boundary_count > 0 || sensitive_count > 0 {
        data.add_section("data_model", format!(
            "Data model: {} boundaries detected, {} sensitive fields classified.",
            boundary_count, sensitive_count,
        ));
    }

    // ── coupling: top coupled modules ────────────────────────────────
    let coupling = rt.storage.with_reader(|conn| {
        conn.prepare_cached(
            "SELECT module_name, instability, abstractness, distance_from_main_sequence
             FROM coupling_metrics ORDER BY distance_from_main_sequence DESC LIMIT 10"
        )
        .and_then(|mut s| {
            let rows = s.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?, r.get::<_, f64>(3)?))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
        })
        .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or_default();

    if !coupling.is_empty() {
        let lines: Vec<String> = coupling.iter().map(|(name, inst, abs, dist)| {
            format!("- {} (I={:.2}, A={:.2}, D={:.2})", name, inst, abs, dist)
        }).collect();
        data.add_section("coupling", format!(
            "Coupling analysis (top {} by distance from main sequence):\n{}",
            lines.len(), lines.join("\n"),
        ));
    }

    // ── dependencies: same as coupling for now ───────────────────────
    // The "dependencies" section uses coupling data from a dependency perspective.
    if !coupling.is_empty() {
        data.add_section("dependencies", format!(
            "{} modules analyzed for coupling. Highest instability modules may indicate fragile dependencies.",
            coupling.len(),
        ));
    }

    // ── dna: profile summary ─────────────────────────────────────────
    let dna_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM dna_profiles")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    if dna_count > 0 {
        data.add_section("dna", format!(
            "{} DNA gene profiles extracted (naming, spacing, structure, documentation, etc.).",
            dna_count,
        ));
    }

    // ── security: detection summary by category ──────────────────────
    let security_detections = rt.storage.with_reader(|conn| {
        conn.prepare_cached(
            "SELECT category, COUNT(*) as cnt FROM detections WHERE category = 'Security' OR category = 'Auth'
             GROUP BY category ORDER BY cnt DESC"
        )
        .and_then(|mut s| {
            let rows = s.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
        })
        .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or_default();

    if !security_detections.is_empty() {
        let lines: Vec<String> = security_detections.iter().map(|(cat, cnt)| {
            format!("- {}: {} detections", cat, cnt)
        }).collect();
        data.add_section("security", format!(
            "Security-related detections:\n{}", lines.join("\n"),
        ));
    }

    // ── owasp_cwe: OWASP analysis summary ────────────────────────────
    let owasp_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM owasp_analysis")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    if owasp_count > 0 {
        data.add_section("owasp_cwe", format!(
            "{} OWASP Top 10 findings mapped to CWE identifiers.", owasp_count,
        ));
    }

    // ── crypto: cryptographic findings ───────────────────────────────
    let crypto_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM crypto_findings")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    if crypto_count > 0 {
        data.add_section("crypto", format!(
            "{} cryptographic findings (weak hashes, insecure algorithms, key management issues).",
            crypto_count,
        ));
    }

    // ── test_topology / test_requirements: test quality ──────────────
    let test_quality = rt.storage.with_reader(|conn| {
        conn.prepare_cached(
            "SELECT function_id, overall_score, smells FROM test_quality ORDER BY overall_score ASC LIMIT 10"
        )
        .and_then(|mut s| {
            let rows = s.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, Option<String>>(2)?))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
        })
        .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or_default();

    if !test_quality.is_empty() {
        let aggregate = test_quality.iter().find(|(id, _, _)| id == "__aggregate__");
        let section = if let Some((_, score, _)) = aggregate {
            format!("Test quality score: {:.1}/100.", score * 100.0)
        } else {
            let avg: f64 = test_quality.iter().map(|(_, s, _)| s).sum::<f64>() / test_quality.len() as f64;
            format!("Test quality: {} functions analyzed, average score {:.1}/100.", test_quality.len(), avg * 100.0)
        };
        data.add_section("test_topology", section.clone());
        data.add_section("test_requirements", section);
    }

    // ── constraints: constraint verification summary ─────────────────
    let constraint_count = rt.storage.with_reader(|conn| {
        conn.prepare_cached("SELECT COUNT(*) FROM constraints")
            .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
            .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or(0);

    if constraint_count > 0 {
        let verified = rt.storage.with_reader(|conn| {
            conn.prepare_cached("SELECT COUNT(*) FROM constraint_verifications WHERE passed = 1")
                .and_then(|mut s| s.query_row([], |r| r.get::<_, i64>(0)))
                .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
        }).unwrap_or(0);
        data.add_section("constraints", format!(
            "{} constraints defined, {} verified as passing.", constraint_count, verified,
        ));
    }

    // ── data_flow: combines taint + boundary info ────────────────────
    if !taint_summary.is_empty() || boundary_count > 0 {
        data.add_section("data_flow", format!(
            "Data flow: {} taint flow paths traced, {} data boundaries identified.",
            taint_summary.len(), boundary_count,
        ));
    }

    // ── business_logic: pattern summary by category ──────────────────
    let category_counts = rt.storage.with_reader(|conn| {
        conn.prepare_cached(
            "SELECT category, COUNT(*) as cnt FROM detections GROUP BY category ORDER BY cnt DESC"
        )
        .and_then(|mut s| {
            let rows = s.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
        })
        .map_err(|e| drift_core::errors::StorageError::SqliteError { message: e.to_string() })
    }).unwrap_or_default();

    if !category_counts.is_empty() {
        let lines: Vec<String> = category_counts.iter().map(|(cat, cnt)| {
            format!("- {}: {} detections", cat, cnt)
        }).collect();
        data.add_section("business_logic", format!(
            "Pattern detections by category:\n{}", lines.join("\n"),
        ));
    }

    Ok(())
}

/// Generate a specification document for a module.
#[napi]
pub async fn drift_generate_spec(
    module_json: String,
    migration_path_json: Option<String>,
) -> Result<String> {
    use drift_context::specification::renderer::SpecificationRenderer;
    use drift_context::specification::types::LogicalModule;
    use drift_core::traits::MigrationPath;

    let module: LogicalModule = serde_json::from_str(&module_json)
        .map_err(|e| Error::from_reason(format!("Invalid module JSON: {}", e)))?;

    let migration_path = migration_path_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<MigrationPathInput>(json).ok())
        .map(|mp| MigrationPath::new(
            mp.source_language,
            mp.target_language,
            mp.source_framework,
            mp.target_framework,
        ));

    let renderer = SpecificationRenderer::new();
    let output = renderer.render(&module, migration_path.as_ref());

    let result = serde_json::json!({
        "module_name": output.module_name,
        "sections": output.sections.iter().map(|(s, c)| {
            serde_json::json!({"section": s.name(), "content": c})
        }).collect::<Vec<_>>(),
        "total_token_count": output.total_token_count,
        "has_all_sections": output.has_all_sections(),
    });

    serde_json::to_string(&result)
        .map_err(|e| Error::from_reason(format!("Serialization error: {}", e)))
}

#[derive(serde::Deserialize)]
struct MigrationPathInput {
    source_language: String,
    target_language: String,
    source_framework: Option<String>,
    target_framework: Option<String>,
}
