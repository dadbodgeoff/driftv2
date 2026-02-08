# Drift v2 — Stack Hierarchy (Dependency-Ordered Importance)

> Every system in Drift v2, ranked by structural importance.
> "Importance" = how many other systems break if this one doesn't exist.
> Nothing at Level N can function without Level N-1 being complete.
>
> Updated to reflect PLANNING-DRIFT.md decisions (D1-D7).
> This is a dependency truth map, not a build schedule.

---

## How PLANNING-DRIFT.md Reshapes This Hierarchy

Your planning decisions impose five structural constraints on the hierarchy:

1. **Standalone independence (D1)** — Drift's hierarchy must be self-contained. No system in Drift can depend on Cortex existing. This means the event system (D5) must be designed with no-op defaults from Level 0, not bolted on later. It also means context generation, grounding, and memory-linked tools are strictly optional branches, never on the critical path.

2. **Trait-based event system (D5) is infrastructure, not a feature** — The `DriftEventHandler` trait with no-op defaults is a Level 0 concern. It's the hook point that the bridge crate latches onto. If you don't scaffold it into drift-core from the start, you'll be retrofitting every subsystem later to emit events. This elevates the event system from "nice to have" to "bedrock infrastructure."

3. **Separate databases with ATTACH (D6)** — drift.db is fully self-contained. The storage layer needs to be designed so that ATTACH cortex.db is a read-only overlay, never a dependency. This means every query that touches patterns, call graph, boundaries, etc. must work with drift.db alone. Cross-DB queries are a presentation-layer concern (bridge MCP tools), not an analysis-layer concern.

4. **Bridge crate is a leaf, not a spine (D4)** — cortex-drift-bridge depends on both drift-core and cortex-core but nothing in Drift depends on it. This makes the entire bridge + grounding loop (D7) a Level 5 system — high value, zero structural importance to Drift's own hierarchy. The grounding feedback loop is the killer feature of the *integration*, but Drift doesn't need it to function.

5. **MCP server split aligns with independence (D3)** — drift-analysis server is pure Drift. drift-memory server is bridge-dependent. This means the MCP presentation layer has two tiers of importance: drift-analysis is Level 5 core, drift-memory is Level 5 optional.

---

## Level 0 — Bedrock (Everything Dies Without These)

These are the gravity systems. Every single feature in Drift traces back to one of these.

| System | What It Is | Why It's Bedrock |
|--------|-----------|-----------------|
| **Tree-Sitter Parsers (10 langs)** | AST extraction from source code — functions, classes, imports, exports, calls, decorators, inheritance, access modifiers | Every analysis path starts here. Zero detectors, zero call graph, zero boundaries, zero taint, zero contracts, zero test topology, zero error handling, zero DNA, zero constraints without parsed ASTs. The single most critical system. |
| **Scanner** | Parallel file walking (rayon + walkdir), .gitignore/.driftignore respect, content hashing (xxh3) | The entry point to the entire pipeline. No scanner = no files = no parsing = nothing. Also owns incremental detection — content hashes determine what gets re-analyzed. |
| **SQLite Storage (drift.db)** | Single database, WAL mode, 40+ tables, STRICT mode, Medallion architecture (Bronze/Silver/Gold), CQRS read/write separation | Nothing persists without this. No patterns survive a restart, no call graph is queryable, no incremental analysis is possible. Per D6: drift.db is fully self-contained — ATTACH cortex.db is optional read-only overlay. |
| **NAPI Bridge** | Rust ↔ Node.js interface via napi-rs | The only door between Rust analysis and TS presentation. Without it, Rust computation is trapped. No MCP, no CLI, no VSCode, no LSP. |
| **thiserror Error Enums** | Per-subsystem structured error types | AD6: from the first line of code. Every Rust function returns typed errors. Retrofitting this is brutal — it touches every function signature. |
| **tracing Instrumentation** | Structured logging + span-based timing | AD10: from the first line of code. Every subsystem emits structured events. Without it, you're debugging blind and can't measure performance targets (10K files <3s). |
| **DriftEventHandler Trait** | Trait-based event bus with typed events and no-op defaults | Per D5: this is the hook point the bridge crate latches onto. If subsystems don't emit events from day one (on_scan_complete, on_pattern_approved, on_regression_detected), you retrofit every subsystem later. Zero overhead when no handlers registered. Design it now, consume it later. |
| **Configuration System** | Config loading, env overrides, .driftignore, validation | Everything reads config. Scanner needs ignore patterns, detectors need thresholds, storage needs pragma settings. Must exist before first scan. |

**Dependency truth**: Config + Error Handling + Tracing + Events → Scanner → Parsers → Storage → NAPI → everything else.

The previous hierarchy had 4 bedrock systems. Your planning doc forces 8 — the event trait, error handling, tracing, and config are load-bearing infrastructure that D5/AD6/AD10 explicitly say must exist from line one.

---

## Level 1 — Structural Skeleton (Core Analysis That Most Systems Consume)

These produce the foundational data structures. They don't depend on each other (mostly), but almost everything above depends on at least one.

| System | What It Is | Downstream Consumers | Upstream Dependencies |
|--------|-----------|---------------------|----------------------|
| **Unified Analysis Engine** | 4-phase per-file pipeline: AST pattern detection → string extraction → regex on strings → resolution index building | Detectors, patterns, confidence, outliers, violations, DNA, constraints | Parsers, scanner, string interning |
| **Call Graph Builder** | Function→function edges, 6 resolution strategies, petgraph in-memory + SQLite | Reachability, impact, dead code, taint, error handling propagation, test topology coverage, N+1, contracts (cross-service), simulation, constraints (must_precede/must_follow) | Parsers, storage |
| **Detector System (Trait-Based)** | 16 categories × 3 variants (Base/Learning/Semantic), registry, framework suites | Patterns, violations, confidence, outliers, DNA, constraints, quality gates, audit | Parsers, unified analysis engine |
| **Boundary Detection** | 28+ ORM frameworks, 7 field extractors, sensitive field classification | Security, taint (sinks), reachability (sensitivity), constraints (data_flow), quality gates (security gate) | Parsers, unified language provider |
| **Unified Language Provider** | 9 language normalizers, 20 ORM/framework matchers | Boundary detection, N+1, contract extraction, language intelligence | Parsers |
| **String Interning (lasso)** | ThreadedRodeo for build, RodeoReader for query, PathInterner, FunctionInterner | Unified analysis, call graph, all identifier-heavy systems (60-80% memory reduction) | None (utility) |

**Key insight unchanged**: Call graph feeds ~10 downstream systems. Detector system feeds ~8. Call graph is the highest-leverage "second branch" after the core detection pipeline. But per D1, neither of these systems can assume Cortex exists — they write to drift.db only, and the bridge reads from drift.db via ATTACH or NAPI.

---

## Level 2 — Intelligence Layer (Derived Analysis)

### Tier 2A — Pattern Intelligence (Core Value Prop)

| System | What It Is | Importance |
|--------|-----------|-----------|
| **Bayesian Confidence Scoring** | Beta(1+k, 1+n-k) posterior + momentum, graduated tiers (Established/Emerging/Tentative/Uncertain) | Numerical backbone of the pattern system. Without it: no ranking, no outlier thresholds, no quality gate thresholds. Also: per D7, confidence scores are what the grounding loop compares against Cortex memories — but Drift computes them independently. |
| **Outlier Detection** | Z-Score (n≥30), Grubbs' (10≤n<30), IQR (n<30), rule-based | How Drift finds violations. Patterns without outlier detection = "we know the convention but can't flag deviations." |
| **Pattern Aggregation & Deduplication** | Group by ID, Jaccard similarity (0.85 flag, 0.95 auto-merge), cross-file merging | Turns per-file matches into project-level Pattern entities. |
| **Learning System** | Dominant convention discovery (minOccurrences=3, dominance=0.60, minFiles=2) | What makes Drift adaptive. Without it, every pattern is manually defined. |

**D7 impact on 2A**: The grounding feedback loop (Decision 7) reads confidence scores and pattern data from drift.db to validate Cortex memories. But this is a one-way read — Drift computes confidence independently, the bridge consumes it. No change to internal dependencies, but it means confidence scoring quality directly affects the killer integration feature. Get this right and the grounding loop is powerful. Get it wrong and grounding produces garbage.

### Tier 2B — Graph-Derived Analysis

| System | What It Is | Importance |
|--------|-----------|-----------|
| **Reachability Analysis** | Forward/inverse BFS, sensitivity classification, taint tracking | Powers security ("can user input reach this SQL query?"), impact, taint. High leverage — one engine, many consumers. |
| **Taint Analysis** | Source/sink/sanitizer TOML registry, intraprocedural (P1), interprocedural via summaries (P2) | Single most impactful security improvement. Depends on call graph + reachability + boundaries all being solid. |
| **Impact Analysis** | Transitive caller analysis, risk scoring, dead code, path finding | "What breaks if I change this?" — critical for simulation, CI agent, `drift_impact`. |
| **Error Handling Analysis** | 4-phase: profiling → propagation → unhandled paths → gaps | Depends on call graph for propagation chains. Feeds quality gates, constraints. |
| **Test Topology** | 35+ frameworks, coverage mapping via call graph BFS, minimum test set, mock analysis | Feeds quality gates (test coverage), simulation (test coverage scorer), CI agent. |

### Tier 2C — Structural Intelligence

| System | What It Is | Importance |
|--------|-----------|-----------|
| **Coupling Analysis** | Afferent/efferent, Tarjan's SCC, zones, cycle break suggestions | Architecture health. Consumed by DNA, simulation, quality gates. |
| **Constraint Detection & Verification** | 12 invariant types, 10 categories, AST-based verification | High enforcement value but depends on nearly everything in 2A and 2B as data sources. Consumer, not producer. |
| **Contract Tracking** | BE↔FE matching, path similarity, schema compatibility, breaking changes | Important for full-stack. Self-contained — doesn't feed other systems. |
| **DNA System** | 10 gene extractors, health scoring, mutation detection | Capstone metric. Per D7: DNA health scores are another grounding signal the bridge can compare against Cortex memories, but Drift computes them independently. |
| **Constants & Environment** | Magic numbers, env vars, .env parsing | Narrow scope. Feeds security (secrets) and constraints. |
| **Wrapper Detection** | Thin delegation patterns, clustering | Lowest-impact analysis. Feeds call graph accuracy. |

### Tier 2D — Security Intelligence

| System | What It Is | Importance |
|--------|-----------|-----------|
| **Enterprise Secret Detection** | 100+ patterns, Shannon entropy, contextual scoring, connection strings, base64 | High standalone value. Leaf system — nothing depends on its output except reporting. |
| **OWASP/CWE Mapping** | Every security detector → CWE IDs, OWASP 2025 (9/10 target) | Metadata enrichment. Enterprise compliance. Doesn't change what gets detected. |
| **Cryptographic Failure Detection** | Weak hashes, hardcoded keys, deprecated algorithms, ECB mode | New detector category. Leaf system. |

---

## Level 3 — Enforcement (Consumes Intelligence, Produces Decisions)

| System | What It Is | What It Consumes | Importance |
|--------|-----------|-----------------|-----------|
| **Rules Engine Evaluator** | Pattern matcher → violations → severity → quick fixes (7 strategies) | Patterns, outliers, confidence | Bridge between "detected" and "fix this." Without it, analysis is data with no action. |
| **Quality Gates (6 gates)** | Pattern compliance, constraints, security boundaries, test coverage, error handling, regression | Nearly everything from Level 2 | The enforcement boundary. Makes Drift useful in CI. Without gates, Drift is informational only. |
| **Policy Engine** | 4 built-in policies, 4 aggregation modes, progressive enforcement | Quality gates | Controls strictness. Critical for adoption — too strict and developers disable it. |
| **Audit System** | Health scoring, degradation detection, auto-approve, snapshots | Patterns, confidence, outliers | The "your codebase is drifting" signal. Core value prop. |
| **Violation Feedback Loop (Tricorder-style)** | FP tracking per detector, auto-disable >20% for 30+ days, action feeds back into confidence | Violations, patterns, confidence | Self-healing loop. Per D5: violation feedback events (on_violation_dismissed, on_detector_disabled) should be emitted via DriftEventHandler so the bridge can propagate to Cortex. Design the events now even if the bridge consumes them later. |

**D5 impact on L3**: Every enforcement action that changes state (pattern approved, violation dismissed, gate failed, detector disabled) should emit a typed event via DriftEventHandler. In standalone mode these are no-ops. When the bridge is active, they become Cortex memories. This doesn't change the hierarchy — it just means L3 systems need to call `self.event_handler.on_*()` at the right moments.

---

## Level 4 — Advanced / Capstone Systems

High-value features on top of the full stack. Impressive but they're leaves.

| System | What It Is | Why It's Level 4 |
|--------|-----------|-----------------|
| **Simulation Engine** | 13 task categories, 15 strategies, 4 scorers | Pure consumer of call graph + patterns + constraints + test topology + DNA. |
| **Decision Mining** | 12 categories, git2 integration, ADR detection | Consumes patterns + git history. Doesn't feed core analysis. |
| **Language Intelligence** | 5 normalizers, 5 framework patterns, cross-language queries | Enrichment on parsers + unified provider. |
| **N+1 Query Detection** | Call graph + ORM patterns → loop-query anti-pattern | High value, narrow scope. |
| **Context Generation** | AI-ready context, 11 package managers, token budgeting | Pure consumer. Powers MCP tools. Per D3: this feeds drift-analysis MCP server only — no Cortex dependency. |
| **GAST Normalization** | ~30 normalized node types, per-language normalizers | Optimization of detector system, not new capability. |

---

## Level 5 — Presentation (Pure Consumers)

### 5A — Drift Standalone (No Cortex Dependency)

Per D1 and D3, these work with drift.db alone.

| System | Priority | Rationale |
|--------|----------|-----------|
| **drift-analysis MCP Server** | #1 | How AI agents consume Drift. ~17-20 tools, read-only drift.db, progressive disclosure with 3 entry points. Per D3: this is the standalone Drift MCP server with `drift_*` namespace. |
| **CLI (48-65+ commands)** | #2 | How developers and CI consume Drift. Setup wizard, git integration, reporters. |
| **Quality Gate Reporters (SARIF, GitHub, GitLab, JUnit, HTML)** | #3 | Output formatting. SARIF critical for GitHub Code Scanning. |
| **CI Agent** | #4 | 9 parallel analysis passes, PR-level analysis. |
| **VSCode Extension** | #5 | Editor integration. |
| **LSP Server** | #6 | IDE-agnostic integration. |
| **Dashboard** | #7 | Web visualization. |
| **Galaxy** | #8 | 3D viz. Lowest priority. |

### 5B — Bridge-Dependent Presentation (Requires Cortex + Drift)

Per D1/D3/D4, these only exist when both systems are detected. They are structurally optional — Drift is complete without them.

| System | What It Is | Why It's 5B |
|--------|-----------|-------------|
| **drift-memory MCP Server** | ~15-20 tools, read/write cortex.db + read drift.db, `drift_memory_*` namespace | Per D3: separate from drift-analysis. Only registers when Cortex detected. |
| **Bridge MCP Tools** | `drift_why`, `drift_memory_learn` — tools needing both systems | Per D4: registered conditionally by drift-analysis server when Cortex available. |
| **Grounding Feedback Loop** | Drift scan results validate Cortex memories, confidence adjustment | Per D7: the killer integration feature. But it's a bridge-crate consumer of drift.db data, not a Drift subsystem. Drift doesn't know or care that grounding is happening. |
| **Event-Driven Memory Creation** | Drift events → Cortex memories (pattern:approved → pattern_rationale) | Per D5: the bridge implements DriftEventHandler to create Cortex memories from Drift events. Drift emits events; what happens to them is not Drift's concern. |

**This is the key structural insight from your planning doc**: The grounding loop (D7) is the most valuable feature of the *product*, but it's a leaf in Drift's hierarchy. Drift's job is to compute accurate confidence scores, emit events, and write clean data to drift.db. The bridge's job is to consume that data and make Cortex memories empirically validated. Drift doesn't need to know the bridge exists.

---

## Level 6 — Infrastructure (Parallel / Cross-Cutting)

| System | When Needed | Criticality | Planning Doc Impact |
|--------|------------|-------------|-------------------|
| **Workspace Management** | Before first user interaction | Medium-High | Per D6: must handle drift.db lifecycle. ATTACH cortex.db is workspace-level concern. |
| **Licensing & Feature Gating** | Before public release | Medium | No change from planning doc. |
| **Dual Licensing** | Before public release | Medium | No change. |
| **Telemetry** | Post-launch | Low | No change. |
| **AI Providers** | When explain/fix ships | Low | No change. |
| **Docker Deployment** | When HTTP MCP transport ships | Low | Per D3: need to containerize both MCP servers independently. |
| **CIBench** | When benchmarking | Low | No change. |
| **GitHub Action** | When CI ships | Medium | No change. |

---

## The Critical Path (Updated)

The minimum stack to deliver Drift's core value, incorporating planning decisions:

```
Config + thiserror + tracing + DriftEventHandler trait (scaffolded, no-op defaults)
  → Scanner (file walking, hashing, .driftignore)
    → Parsers (10 langs, tree-sitter)
      → String Interning (lasso — ThreadedRodeo/RodeoReader)
        → Unified Analysis Engine (4-phase AST+regex pipeline)
          → Detector System (16 categories, trait-based)
            → Learning System (dominant convention discovery)
              → Confidence Scoring (Bayesian — Beta posterior + momentum)
                → Outlier Detection (Z-Score/Grubbs'/IQR)
                  → Pattern Aggregation & Dedup
                    → Rules Engine (violations + severity + quick fixes)
                      → Storage (drift.db — standalone, no ATTACH)
                        → NAPI Bridge
                          → CLI (drift scan + drift check)
```

That's 15 systems (was 12 before your planning doc added the infrastructure requirements).

**First branch off the spine**: Call Graph → unlocks reachability, taint, impact, error handling, test topology, constraints.

**Second branch off the spine**: Quality Gates → unlocks CI enforcement, policy engine, audit.

**Third branch (optional, bridge-dependent)**: DriftEventHandler consumers → bridge crate → Cortex memory creation → grounding loop.

The third branch is where D7's killer feature lives, but it's architecturally the last thing that needs to work — because it only consumes what the first two branches produce.

---

## Visual Summary (Updated)

```
  ┌──────────────────────────────────────────────────────────┐
  │              BRIDGE-DEPENDENT (L5B) — Optional            │
  │  drift-memory MCP · Bridge Tools · Grounding Loop         │
  │  Event→Memory · drift_why · drift_memory_learn            │
  │  ← Requires Cortex + Drift both present (D1/D4) →        │
  └────────────────────────────┬─────────────────────────────┘
                               │ (consumes drift.db via ATTACH, D6)
  ┌────────────────────────────┴─────────────────────────────┐
  │              DRIFT STANDALONE (L5A)                        │
  │  drift-analysis MCP · CLI · VSCode · LSP                  │
  │  CI Agent · Dashboard · Galaxy · Reporters                 │
  └────────────────────────────┬─────────────────────────────┘
                               │
  ┌────────────────────────────┴─────────────────────────────┐
  │              ENFORCEMENT (L3)                              │
  │  Rules Engine · Quality Gates · Policy · Audit · Feedback  │
  │  ← All emit DriftEventHandler events (D5) →               │
  └────────────────────────────┬─────────────────────────────┘
                               │
     ┌─────────────────────────┼─────────────────────────────┐
     │                         │                             │
  ┌──┴───────────┐  ┌────────┴────────┐  ┌────────────────┴──┐
  │ PATTERN (2A)  │  │ GRAPH (2B)      │  │ STRUCTURAL (2C)    │
  │ Confidence    │  │ Reachability    │  │ Coupling           │
  │ Outlier       │  │ Taint · Impact  │  │ Constraints · DNA  │
  │ Aggregation   │  │ Errors · Tests  │  │ Contracts          │
  │ Learning      │  │                 │  │ Constants · Wrap   │
  └──────┬───────┘  └────────┬────────┘  └────────┬──────────┘
         │                   │                     │
  ┌──────┴───────────────────┴─────────────────────┘
  │              SKELETON (L1)
  │  Unified Analysis · Call Graph · Detectors
  │  Boundaries · Unified Lang Provider · String Interning
  └──────────────────────┬──────────────────────────┘
                         │
  ┌──────────────────────┴──────────────────────────┐
  │              BEDROCK (L0)                         │
  │  Parsers · Scanner · SQLite (drift.db standalone) │
  │  NAPI · thiserror · tracing · Config              │
  │  DriftEventHandler trait (no-op defaults, D5)     │
  └──────────────────────────────────────────────────┘
```

---

## What Changed From the Previous Hierarchy

| Change | Why | Planning Decision |
|--------|-----|-------------------|
| Bedrock expanded from 4 → 8 systems | thiserror, tracing, config, and DriftEventHandler are all "from first line of code" requirements | AD6, AD10, D5 |
| Event system is Level 0, not Level 3+ | Bridge crate (D4) needs events to exist in drift-core. Retrofitting is worse than scaffolding. | D5 |
| Presentation split into 5A (standalone) and 5B (bridge-dependent) | Drift works alone (D1). Bridge tools are optional. Two MCP servers, not one. | D1, D3, D4 |
| Grounding loop is Level 5B, not Level 2 | It's the killer feature of the integration but a leaf in Drift's hierarchy. Drift computes; bridge consumes. | D4, D7 |
| drift.db explicitly standalone | ATTACH cortex.db is optional read-only overlay. Every query works without it. | D6 |
| Critical path grew from 12 → 15 systems | Infrastructure requirements (error handling, tracing, events, config) are now explicit. | AD6, AD10, D5 |
| Confidence scoring importance elevated | It's not just internal — it's the primary data the grounding loop (D7) validates against. Quality here = grounding quality. | D7 |
| No system in Drift imports from cortex-core | Structural independence enforced at every level. Bridge is the ONLY cross-import point. | D1, D4 |

---

*This hierarchy reflects both structural dependency truth and the architectural constraints from PLANNING-DRIFT.md.*
*Drift is self-contained. The bridge is a leaf. The grounding loop is the killer feature that Drift enables but doesn't own.*
