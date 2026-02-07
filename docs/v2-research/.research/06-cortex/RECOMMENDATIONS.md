# 06 Cortex Memory System — Recommendations

> Concrete improvement recommendations for Drift v2's Cortex memory system, derived from the v1 recap and targeted external research (R1-R15).

---

## CX1: Hybrid Search — FTS5 + sqlite-vec with Reciprocal Rank Fusion

**Priority**: P0
**Evidence**: R2 (Simon Willison, Microsoft Azure), R8 (sqlite-vec best practices)

Replace vector-only retrieval with hybrid search combining FTS5 full-text search and sqlite-vec vector similarity, fused via Reciprocal Rank Fusion (RRF).

**Why**: Vector search misses exact keyword matches (function names, pattern IDs, specific terms like "bcrypt"). Full-text search misses semantic meaning. RRF combines both without score normalization: `score = Σ 1/(60 + rank_i)`.

**Implementation**:
- Add FTS5 virtual table on memory content + summary + tags
- Run both FTS5 and vector queries in parallel
- Fuse results with RRF (k=60)
- Pre-filter by type/importance before both searches to reduce candidate set

---

## CX2: Code-Specific Embedding Model

**Priority**: P0
**Evidence**: R3 (Modal benchmarks, CodeXEmbed paper, Jina Code, Qodo Embed)

Replace general-purpose 384-dim Transformers.js embeddings with a code-specific model.

**Recommended models**:
- Local: Jina Code Embeddings v2 (137M params, Apache 2.0, 8192 context) or CodeRankEmbed (137M, MIT, 8192 context)
- API: VoyageCode3 (32K context, 2048 dims, 300+ languages)
- Matryoshka support: Store 1024-dim, use 384-dim for fast search, full dims for re-ranking

**Rust migration**: Use `ort` crate (ONNX Runtime) for 3-5x speedup over Transformers.js (R4).

---

## CX3: Embedding Enrichment

**Priority**: P1
**Evidence**: R14 (RAG optimization), R8 (sqlite-vec best practices)

Prepend structured metadata to memory content before embedding generation:
```
[tribal|critical|security] Never call the payment API without idempotency keys.
Files: src/payments/api.ts, src/checkout/service.ts
Patterns: payment-api-pattern, idempotency-pattern
```

This gives the embedding model more signal about memory context, improving similarity search for related queries. One-time cost at embedding time.

---

## CX4: Two-Phase Memory Pipeline (Mem0-Inspired)

**Priority**: P1
**Evidence**: R1 (Mem0 paper — 26% improvement over OpenAI memory, 91% lower p95 latency)

Add explicit deduplication/update phase before memory storage:
1. **Extraction phase**: Identify salient facts from interaction
2. **Update phase**: Compare each candidate against existing memories via vector similarity → LLM determines ADD, UPDATE, DELETE, or NOOP

This prevents memory bloat and ensures consistency. Currently Cortex creates memories directly without checking for near-duplicates.

---

## CX5: Graph-Based Memory Layer

**Priority**: P2
**Evidence**: R1 (Mem0g graph variant), R11 (CausalKG paper)

Add optional entity-relationship graph where nodes are entities (with types, embeddings, metadata) and edges are typed relationships as triplets (source, relation, destination). Enables multi-hop reasoning that flat memory stores cannot support.

**Implementation**: Use `petgraph::StableGraph` in Rust (R5) synced with SQLite causal_edges table. StableGraph handles frequent add/remove of edges. Built-in Tarjan's SCC detects circular causal chains.

---

## CX6: Retrieval Re-Ranking Stage

**Priority**: P1
**Evidence**: R10 (RAG production best practices)

Add a two-stage retrieval pipeline:
1. **Fast retrieval**: Hybrid search (CX1) returns top-K candidates (K=50)
2. **Precise re-ranking**: Cross-encoder or lightweight LLM scores each candidate against the query

This significantly improves precision. The re-ranker can be a small model (e.g., cross-encoder from sentence-transformers) running locally via `ort`.

---

## CX7: Accurate Token Counting

**Priority**: P0
**Evidence**: R12 (tiktoken, tiktoken-rs)

Replace string-length approximation with actual tokenizer-based counting. Use `tiktoken-rs` in Rust, `tiktoken` or `js-tiktoken` in TypeScript. Cache token counts per memory (they don't change unless content changes).

**Impact**: Prevents budget overflows (truncation) and underutilization (wasted context window).

---

## CX8: Evidence-Based Memory Promotion

**Priority**: P1
**Evidence**: R15 (Governed Memory Fabric), R7 (neuroscience-inspired consolidation)

Replace time-only consolidation triggers with evidence-based promotion thresholds:
- Memory promoted to semantic only if confirmed by ≥2 episodes, validated by user feedback, or supported by pattern data
- Add retrieval-difficulty triggers: if a memory that should be relevant keeps scoring low, it needs reinforcement or embedding refresh
- Per-memory adaptive decay rates based on access patterns (not just type-based half-lives)

---

## CX9: Expanded Privacy Patterns

**Priority**: P0
**Evidence**: R9 (Elastic PII detection, layered approach)

Expand from 10 patterns to 50+:
- All provider-specific secrets from Rust core (Azure keys, GCP service accounts, npm/PyPI tokens, Slack tokens, GitHub tokens)
- Connection strings (PostgreSQL, MySQL, MongoDB, Redis URLs with embedded passwords)
- Base64-encoded secrets
- Hardcoded IPs in configuration
- Consider NER for unstructured PII in tribal/meeting/conversation memories

---

## CX10: Memory System Observability

**Priority**: P1
**Evidence**: R13 (Salesforce system-level AI, enterprise RAG maintenance)

Extend `getHealth()` to enterprise-grade observability:
- Retrieval effectiveness: was the retrieved memory actually used by the AI?
- Token efficiency: how much of the budget was useful vs wasted?
- Memory quality trends over time: is the system getting smarter or degrading?
- Audit trail for all memory mutations (create, update, archive, confidence changes)
- Query timing, cache hit rates, embedding latency

---

## CX11: Causal Graph Improvements

**Priority**: P2
**Evidence**: R11 (CausalKG paper), R5 (petgraph)

- Enforce DAG constraint — detect and handle cycles
- Add counterfactual queries: "What would have happened if we hadn't adopted this pattern?"
- Add intervention queries: "If we change this convention, what memories become invalid?"
- Version causal edges for evolution tracking
- Consider LLM-assisted causal discovery to augment heuristic strategies

---

## CX12: Concurrent Caching with Moka

**Priority**: P1
**Evidence**: R6 (moka crate — TinyLFU + LRU, thread-safe)

Replace L1 in-memory Map with `moka::sync::Cache`:
- TinyLFU provides better hit ratio than simple LRU
- Per-entry TTL enables adaptive expiration (prediction cache: short TTL, embedding cache: long TTL)
- Size-aware eviction prevents memory bloat from large embeddings
- Thread-safe without external locking

---

## CX13: Query Expansion for Improved Recall

**Priority**: P2
**Evidence**: R10 (RAG production best practices)

Generate 2-3 query variants before searching:
- Original query
- Rephrased with synonyms/related terms
- Hypothetical Document Embedding (HyDE): generate a hypothetical answer and embed that

This bridges the gap between query style and memory content style, improving recall for memories that use different terminology than the query.

---

## Summary Table

| # | Recommendation | Priority | Evidence |
|---|---------------|----------|----------|
| CX1 | Hybrid search (FTS5 + sqlite-vec + RRF) | P0 | R2, R8 |
| CX2 | Code-specific embedding model | P0 | R3, R4 |
| CX3 | Embedding enrichment with metadata | P1 | R14, R8 |
| CX4 | Two-phase memory pipeline (Mem0-inspired) | P1 | R1 |
| CX5 | Graph-based memory layer | P2 | R1, R11 |
| CX6 | Retrieval re-ranking stage | P1 | R10 |
| CX7 | Accurate token counting (tiktoken) | P0 | R12 |
| CX8 | Evidence-based memory promotion | P1 | R15, R7 |
| CX9 | Expanded privacy patterns (50+) | P0 | R9 |
| CX10 | Memory system observability | P1 | R13 |
| CX11 | Causal graph improvements | P2 | R11, R5 |
| CX12 | Concurrent caching with moka | P1 | R6 |
| CX13 | Query expansion for improved recall | P2 | R10 |

**Why hybrid**: Vector-only search misses exact keyword matches (e.g., searching for "bcrypt" might return memories about "password hashing" but miss the one that literally says "use bcrypt"). FTS5 catches these. RRF combines both without requiring score normalization.

**Evidence**:
- Hybrid search with RRF: https://simonwillison.net/2024/Oct/4/hybrid-full-text-search-and-vector-search-with-sqlite/
- Azure hybrid search: https://learn.microsoft.com/en-us/azure/search/hybrid-search-overview
- sqlite-vec: https://github.com/asg017/sqlite-vec

---

### FA2: Code-Specific Embedding Model

**Priority**: P0 (Build First)
**Effort**: Medium
**Impact**: Determines retrieval quality for all memory operations

**What to Build**:
Replace the general-purpose 384-dim Transformers.js model with a code-specific embedding model. Support multiple dimensions via Matryoshka representation.

**Provider hierarchy**:
1. **Local (default)**: Jina Code Embeddings v2 (137M params, Apache 2.0, 8192 context) via ONNX Runtime. Store 1024-dim embeddings. Use 384-dim truncation for fast search, full 1024-dim for re-ranking.
2. **API (optional)**: VoyageCode3 (32K context, 2048 dims, 300+ languages). For teams that want maximum quality.
3. **Fallback**: all-MiniLM-L6-v2 via Transformers.js (current behavior, for air-gapped environments without ONNX).

**Embedding enrichment**: Before embedding, prepend structured metadata:
```
[{type}|{importance}|{category}] {summary}
Files: {linkedFiles}
Patterns: {linkedPatterns}
```
This gives the embedding model more signal for discriminative representations.

**Evidence**:
- Code embedding comparison: https://modal.com/blog/6-best-code-embedding-models-compared
- Jina Code: https://jina.ai/models/jina-code-embeddings-1.5b/
- Embedding enrichment: https://hyperion-consulting.io/en/insights/rag-optimization-production-2026-best-practices

---

### FA3: Structured Error Handling and Audit Trail

**Priority**: P0 (Build First)
**Effort**: Low
**Impact**: Every subsystem uses this — impossible to retrofit

**What to Build**:
Every memory mutation (create, update, archive, confidence change, link, unlink) is logged to an append-only audit table. Every error uses structured error types.

```sql
CREATE TABLE memory_audit_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  memory_id TEXT NOT NULL,
  operation TEXT NOT NULL,  -- create|update|archive|restore|link|unlink|decay|validate|consolidate
  details TEXT,             -- JSON: what changed
  actor TEXT,               -- system|user|consolidation|decay|validation|learning
  timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_audit_memory ON memory_audit_log(memory_id);
CREATE INDEX idx_audit_timestamp ON memory_audit_log(timestamp);
```

For Rust:
```rust
#[derive(thiserror::Error, Debug)]
pub enum CortexError {
    #[error("memory not found: {id}")]
    MemoryNotFound { id: String },
    #[error("invalid memory type: {type_name}")]
    InvalidType { type_name: String },
    #[error("embedding failed: {0}")]
    EmbeddingError(#[from] EmbeddingError),
    #[error("storage error: {0}")]
    StorageError(#[from] rusqlite::Error),
    #[error("causal cycle detected: {path}")]
    CausalCycle { path: String },
    #[error("token budget exceeded: needed {needed}, available {available}")]
    TokenBudgetExceeded { needed: usize, available: usize },
}
```

**Evidence**:
- Governed memory fabric: https://www.csharp.com/article/the-gdel-autonomous-memory-fabric-db-layer-the-database-substrate-that-makes-c/
- thiserror: https://docs.rs/thiserror

---

## Phase 1: Storage & Embedding Core

Build the foundational data layer that everything else depends on.

### R1: Memory Storage with Bitemporal Tracking

**Priority**: P0
**Effort**: High

**What to Build**:
SQLite storage implementing the full `IMemoryStorage` interface from v1, with these v2 enhancements:

1. All 23 memory types with typed content (serde serialization, not JSON blobs)
2. Bitemporal tracking: transaction_time (when recorded) + valid_time (when true)
3. Relationship system with 13 relationship types and strength scoring
4. Link tables: memory_patterns, memory_constraints, memory_files (with citations), memory_functions
5. FTS5 index for keyword search (FA1)
6. Vector table for semantic search (FA1)
7. Audit log for all mutations (FA3)
8. WAL mode, NORMAL synchronous, 256MB mmap

**Key difference from v1**: Content stored as typed Rust structs (via serde), not JSON blobs. This enables compile-time validation and faster deserialization.

**Dependencies**: `rusqlite` (bundled SQLite), `serde` + `serde_json`, `chrono`, `uuid`

---

### R2: Embedding Engine with ONNX Runtime

**Priority**: P0
**Effort**: High

**What to Build**:
Multi-provider embedding system with code-specific models and 3-tier caching.

**Architecture**:
```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn dimensions(&self) -> usize;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
    fn is_available(&self) -> bool;
}
```

**Providers**:
1. `OnnxProvider` — Loads ONNX models via `ort` crate. Default: Jina Code v2 (1024-dim). Supports quantized models (INT8) for faster inference.
2. `OpenAIProvider` — API-based, for teams wanting maximum quality.
3. `OllamaProvider` — Local Ollama instance.
4. `FallbackProvider` — Simple TF-IDF based embeddings for air-gapped environments with no ML runtime.

**3-Tier Cache**:
- L1: `moka::sync::Cache` with size-aware eviction (embedding vectors are large)
- L2: SQLite table with content-hash keys
- L3: Memory-mapped precomputed embeddings for frequently-accessed content

**Embedding enrichment** (FA2): Prepend metadata before embedding.

**Matryoshka support**: Store full-dimension embeddings. Truncate to lower dimensions for fast search. Use full dimensions for re-ranking.

**Evidence**:
- ort crate: https://ort.pyke.io/
- Rust ONNX benchmarks: https://markaicode.com/rust-onnx-ml-models-2025/
- moka cache: https://docs.rs/moka/latest/moka/

---

### R3: Accurate Token Counting

**Priority**: P0
**Effort**: Low

**What to Build**:
Replace string-length approximation with actual tokenizer-based counting.

Use `tiktoken-rs` for cl100k_base (GPT-4/Claude compatible) tokenization. Cache token counts per memory (they don't change unless content changes).

```rust
use tiktoken_rs::cl100k_base;

pub struct TokenCounter {
    bpe: tiktoken_rs::CoreBPE,
    cache: moka::sync::Cache<String, usize>,  // content_hash -> token_count
}

impl TokenCounter {
    pub fn count(&self, text: &str) -> usize {
        let hash = blake3::hash(text.as_bytes()).to_hex().to_string();
        self.cache.get_with(hash, || self.bpe.encode_ordinary(text).len())
    }
}
```

**Key difference from v1**: v1 used `text.length / 4` approximation. v2 uses exact tokenization with caching.

**Evidence**:
- tiktoken-rs: https://docs.rs/tiktoken-rs/

---

## Phase 2: Retrieval & Search

### R4: Hybrid Retrieval Engine with Re-Ranking

**Priority**: P0
**Effort**: High

**What to Build**:
A two-stage retrieval pipeline: fast candidate gathering → precise re-ranking.

**Stage 1 — Candidate Gathering** (fast, broad):
1. Pre-filter by memory type based on intent weighting (same as v1)
2. Pre-filter by importance (skip low-importance for tight budgets)
3. Run hybrid search (FA1): FTS5 + sqlite-vec with RRF fusion
4. Gather additional candidates by linked entities (patterns, files, functions)
5. Deduplicate candidates

**Stage 2 — Re-Ranking** (precise, narrow):
1. Score each candidate with multi-factor relevance scorer:
   - Semantic similarity (from vector search, already computed)
   - Keyword match score (from FTS5, already computed)
   - File proximity (same file/directory as active context)
   - Pattern alignment (linked to relevant patterns)
   - Recency (last accessed, last validated)
   - Confidence level
   - Importance level
   - Intent-type match (boosted types for current intent)
2. Apply session deduplication (skip already-sent memories)
3. Compress to fit token budget using hierarchical compression
4. Return with metadata (scores, compression levels, token counts)

**Query expansion**: For the focus string, generate 2-3 variants:
- Original: "authentication middleware"
- Variant 1: "auth middleware guard interceptor"
- Variant 2: "login session token verification"
Run all variants through hybrid search, merge results.

**Evidence**:
- RAG re-ranking: https://hyperion-consulting.io/en/insights/rag-optimization-production-2026-best-practices
- Hybrid search: https://learn.microsoft.com/en-us/azure/search/hybrid-search-overview

---

### R5: 4-Level Hierarchical Compression with Accurate Budgeting

**Priority**: P0
**Effort**: Medium

**What to Build**:
Same 4-level compression as v1, but with accurate token counting (R3) and smarter packing.

**Levels**: Level 0 (IDs, ~5 tokens), Level 1 (one-liners, ~50 tokens), Level 2 (with examples, ~200 tokens), Level 3 (full context, ~500+ tokens).

**Packing algorithm**: Replace greedy approach with a priority-weighted bin-packing:
1. Sort memories by `importance × relevance_score` (descending)
2. For each memory, try Level 3 → 2 → 1 → 0 until it fits remaining budget
3. Critical memories always get at least Level 1
4. Track actual token counts (R3), not estimates

**Key difference from v1**: Accurate token counting prevents budget overflows. Priority-weighted packing ensures the most important memories get the most detail.

---

### R6: Intent-Aware Retrieval with Expanded Intent Taxonomy

**Priority**: P1
**Effort**: Medium

**What to Build**:
Expand the intent taxonomy and make intent weighting configurable.

**V2 Intent Taxonomy** (18 intents):
- Domain-agnostic: create, investigate, decide, recall, learn, summarize, compare
- Code-specific: add_feature, fix_bug, refactor, security_audit, understand_code, add_test, review_code, deploy, migrate
- Universal: spawn_agent, execute_workflow, track_progress

**Intent → Type Boost Matrix** (configurable via TOML):
```toml
[intents.fix_bug]
tribal = 1.5
incident = 1.8
code_smell = 1.5
episodic = 1.3
feedback = 1.2

[intents.security_audit]
tribal = 1.8
incident = 1.5
constraint_override = 1.3
pattern_rationale = 1.2
```

**Key difference from v1**: Configurable weights, expanded taxonomy, and the boost matrix is data-driven (can be tuned based on retrieval effectiveness metrics).

---

## Phase 3: Knowledge Management

### R7: Sleep-Inspired Consolidation with Evidence-Based Promotion

**Priority**: P1
**Effort**: High

**What to Build**:
Same 5-phase consolidation pipeline as v1, with these enhancements:

1. **Evidence-based promotion**: Episodic memories are only promoted to semantic if they have ≥3 supporting episodes OR have been confirmed by user feedback OR are linked to approved patterns. Time alone is not sufficient.

2. **Retrieval-difficulty trigger**: If a memory that should be relevant (high importance, linked to active patterns) consistently scores low in retrieval, trigger consolidation to refresh its embedding or merge it with supporting context.

3. **Non-LLM fallback for abstraction**: The abstraction phase (extracting generalizable patterns from episodes) should have a rule-based fallback for air-gapped environments:
   - Group episodes by linked files/patterns
   - Extract common keywords and phrases
   - Create semantic memory with merged content
   - Flag for human review

4. **Adaptive scheduling** (same as v1): Token pressure, memory count, confidence degradation, contradiction density triggers.

**Evidence**:
- Evidence-based promotion: https://www.csharp.com/article/the-gdel-autonomous-memory-fabric-db-layer-the-database-substrate-that-makes-c/
- Retrieval-difficulty triggers: https://arxiv.org/html/2503.18371
- Spaced repetition principles: https://link.springer.com/chapter/10.1007%2F978-3-030-52240-7_65

---

### R8: Multi-Factor Decay with Adaptive Half-Lives

**Priority**: P1
**Effort**: Low

**What to Build**:
Same 5-factor decay formula as v1, with per-memory adaptive half-lives.

**Enhancement**: Instead of fixed type-based half-lives, compute per-memory adaptive half-lives:
```
adaptiveHalfLife = baseHalfLife × accessFrequencyFactor × validationFactor × linkageFactor
```
- `accessFrequencyFactor`: Frequently accessed memories decay slower (1.0 - 2.0×)
- `validationFactor`: Recently validated memories decay slower (1.0 - 1.5×)
- `linkageFactor`: Memories linked to active patterns/constraints decay slower (1.0 - 1.3×)

This means a tribal memory that's accessed daily and linked to active patterns might have an effective half-life of 365 × 2.0 × 1.5 × 1.3 = 1,423 days, while an unlinked, rarely-accessed tribal memory decays at the base 365 days.

**Evidence**:
- Adaptive forgetting curves: https://link.springer.com/chapter/10.1007%2F978-3-030-52240-7_65
- Human-like forgetting: https://arxiv.org/html/2506.12034v2

---

### R9: Contradiction Detection with Graph Propagation

**Priority**: P1
**Effort**: Medium

**What to Build**:
Same contradiction detection as v1, with these enhancements:

1. **Semantic contradiction detection**: Use embedding similarity + negation pattern matching (same as v1). Add: cross-reference with linked patterns — if two memories link to the same pattern but have opposing content, flag as contradiction.

2. **Confidence propagation via in-memory graph**: Maintain a `petgraph::StableGraph` in memory (synced with SQLite). When a contradiction is detected, propagate confidence changes through the graph using BFS with decay factor. This is O(V+E) instead of repeated SQLite queries.

3. **Consensus detection**: When ≥3 memories support the same conclusion, boost all of them (+0.2) and mark as consensus. Consensus memories resist contradiction from single opposing memories.

4. **Temporal supersession**: Automatically detect when a newer memory supersedes an older one on the same topic. Use embedding similarity + temporal ordering.

**Propagation rules** (same as v1):
- Direct contradiction: -0.3
- Partial contradiction: -0.15
- Supersession: -0.5
- Confirmation: +0.1
- Consensus (≥3 supporters): +0.2
- Supporting propagation factor: 0.5×
- Archival threshold: 0.15

**Evidence**:
- petgraph for graph operations: https://docs.rs/petgraph/
- Mem0 contradiction handling: https://arxiv.org/html/2504.19413

---

### R10: 4-Dimension Validation with Healing

**Priority**: P1
**Effort**: Medium

**What to Build**:
Same 4-dimension validation as v1 (citation, temporal, contradiction, pattern alignment), with these enhancements:

1. **Citation validation**: Check file existence + content hash. NEW: If file was renamed/moved (detected via git), auto-update the citation instead of flagging as stale.

2. **Temporal validation**: Check validUntil expiry. NEW: For memories linked to specific code versions (git commits), check if the code has changed significantly since the memory was created.

3. **Contradiction validation**: Run contradiction detector. NEW: Check for consensus — if a memory has consensus support, it's more resistant to contradiction.

4. **Pattern alignment**: Check if linked patterns still exist and are still approved. NEW: If a pattern was removed or its confidence dropped significantly, flag linked memories for review.

**Healing strategies** (enhanced):
- Confidence adjustment (same as v1)
- Citation auto-update via git rename detection (NEW)
- Embedding refresh — re-embed memories whose content context has changed (NEW)
- Archival with reason tracking (same as v1)
- Flagging for human review (same as v1)

---

## Phase 4: Causal Intelligence

### R11: Causal Graph with DAG Enforcement

**Priority**: P1
**Effort**: High

**What to Build**:
Full causal system from v1, with these enhancements:

1. **In-memory graph**: Maintain a `petgraph::StableGraph<CausalNode, CausalEdge>` synced with SQLite. All traversals operate on the in-memory graph for speed. SQLite is the persistence layer.

2. **DAG enforcement**: Detect cycles before adding causal edges. If a cycle would be created, reject the edge or flag for review. Use petgraph's built-in cycle detection.

3. **8 relation types** (same as v1): caused, enabled, prevented, contradicts, supersedes, supports, derived_from, triggered_by.

4. **6 inference strategies** (same as v1): temporal proximity, semantic similarity, entity overlap, explicit reference, pattern matching, file co-occurrence. Weighted scoring with configurable weights.

5. **Narrative generation**: Template-based narrative builder that produces human-readable "why" explanations from causal chains. Include confidence scores and evidence references.

6. **Counterfactual queries** (NEW): "If we hadn't adopted pattern X, what memories would be affected?" — traverse the causal graph from the pattern's linked memories and identify all downstream effects.

7. **Intervention queries** (NEW): "If we change convention X, what needs to be updated?" — identify all memories causally dependent on the convention.

**Evidence**:
- petgraph: https://docs.rs/petgraph/
- Causal knowledge graphs: https://www.researchgate.net/publication/357765711_CausalKG_Causal_Knowledge_Graph_Explainability_using_interventional_and_counterfactual_reasoning

---

### R12: "Why" System with Causal Narratives

**Priority**: P1
**Effort**: Medium

**What to Build**:
The "killer feature" — synthesizes complete explanations of WHY things are the way they are.

**Pipeline**:
1. Gather pattern rationales for the focus area
2. Gather decision contexts (ADRs, decision memories)
3. Gather tribal knowledge (warnings, consequences)
4. Gather code smells (anti-patterns to avoid)
5. Traverse causal graph from relevant memories (R11)
6. Generate narrative from causal chains
7. Aggregate warnings from all sources
8. Compress to fit token budget

**Output**:
```rust
pub struct WhyContext {
    pub patterns: Vec<PatternContext>,
    pub decisions: Vec<DecisionContext>,
    pub tribal: Vec<TribalContext>,
    pub anti_patterns: Vec<AntiPatternContext>,
    pub narrative: Option<CausalNarrative>,
    pub warnings: Vec<Warning>,
    pub summary: String,
    pub confidence: f64,
    pub token_count: usize,
}
```

**Key difference from v1**: Integrated counterfactual reasoning ("what would happen if...") and intervention analysis ("what needs to change if...").

---

## Phase 5: Learning & Prediction

### R13: Correction Analysis with Principle Extraction

**Priority**: P1
**Effort**: High

**What to Build**:
Full learning pipeline from v1 with these enhancements:

1. **10 correction categories** (same as v1): pattern_violation, tribal_miss, constraint_violation, style_preference, naming_convention, architecture_mismatch, security_issue, performance_issue, api_misuse, other.

2. **Category → Memory Type mapping** (same as v1): pattern_violation→pattern_rationale, tribal_miss→tribal, security_issue→tribal(critical), etc.

3. **Diff analysis**: Compare original vs corrected code. Extract additions, removals, modifications, semantic changes.

4. **Principle extraction**: Generalize the correction into a reusable rule. For air-gapped environments, use rule-based extraction (keyword matching, pattern templates). For connected environments, optionally use LLM for higher-quality extraction.

5. **Automatic causal inference**: When a correction creates a new memory, automatically infer causal relationships with existing memories (R11).

6. **Deduplication before storage** (NEW, inspired by Mem0): Before creating a new memory from a correction, check for existing memories with high similarity. If found, UPDATE the existing memory instead of creating a duplicate. Use the same ADD/UPDATE/NOOP decision pattern as Mem0.

**Evidence**:
- Mem0 extraction/update pipeline: https://arxiv.org/html/2504.19413

---

### R14: Active Learning Loop

**Priority**: P2
**Effort**: Medium

**What to Build**:
Same active learning loop as v1:

1. Identify memories needing validation (low confidence + high importance, old + never validated, contradicted but unresolved)
2. Generate validation prompts for the user
3. Process feedback (confirm/reject/modify)
4. Update confidence based on response
5. Store validation feedback for calibration

**Enhancement**: Prioritize validation candidates by impact — memories that are frequently retrieved but have uncertain confidence should be validated first (they affect the most AI interactions).

---

### R15: Predictive Memory Preloading

**Priority**: P2
**Effort**: Medium

**What to Build**:
Same 4-strategy prediction system as v1:

1. **FileBasedPredictor**: Memories linked to active file and its imports
2. **PatternBasedPredictor**: Memories linked to detected patterns in active file
3. **TemporalPredictor**: Time-of-day and day-of-week patterns from usage history
4. **BehavioralPredictor**: Recent queries, intents, frequent memories

**Enhancements**:
- **Adaptive TTL**: Instead of fixed 5-minute cache TTL, adapt based on file change frequency. Rapidly changing files get shorter TTL.
- **Git-aware prediction**: If on a feature branch, predict memories related to the feature's domain (from branch name and recent commits).
- **Pre-embed queries**: For predicted memories, pre-compute the hybrid search results so retrieval is instant when the query arrives.

**Dependencies**: `moka` for prediction cache with per-entry TTL.

---

## Phase 6: Integration & Observability

### R16: Session Management with Token Efficiency Tracking

**Priority**: P1
**Effort**: Low

**What to Build**:
Same session management as v1 (deduplication, token tracking, cleanup), with enhanced observability:

1. **Deduplication** (same as v1): Track loaded memories per session. Skip already-sent memories. 30-50% token savings.

2. **Token efficiency metrics** (NEW): Track per-session:
   - `tokens_sent`: Total tokens sent to AI
   - `tokens_useful`: Tokens from memories that were actually referenced in AI output (requires feedback)
   - `efficiency_ratio`: useful / sent
   - `deduplication_savings`: Tokens saved by not re-sending

3. **Session analytics** (NEW): Aggregate across sessions to identify:
   - Most frequently retrieved memories (candidates for pinning)
   - Least useful memories (candidates for archival)
   - Intent distribution (what are users doing most?)
   - Average retrieval latency by intent type

---

### R17: Privacy Sanitization with Expanded Patterns

**Priority**: P1
**Effort**: Low-Medium

**What to Build**:
Expand from 10 patterns to 50+ patterns, organized by category:

**PII Patterns** (15+):
- Email, phone, SSN, credit card, IP address (same as v1)
- NEW: Passport numbers, driver's license, date of birth, physical addresses, national ID numbers

**Secret Patterns** (35+):
- API keys, AWS keys, JWT, private keys, passwords (same as v1)
- NEW: Azure keys, GCP service accounts, GitHub tokens (ghp_, gho_, ghs_), GitLab tokens (glpat-), npm tokens, PyPI tokens, Slack tokens (xoxb-, xoxp-), Stripe keys (sk_live_, pk_live_), Twilio tokens, SendGrid keys, Heroku API keys, DigitalOcean tokens, Datadog API keys
- NEW: Connection strings (PostgreSQL, MySQL, MongoDB, Redis URLs with embedded credentials)
- NEW: Base64-encoded secrets (detect base64 strings assigned to sensitive variables)

**Context-aware scoring** (NEW):
- In test file: -0.20 confidence
- In comment: -0.30 confidence
- In .env file: +0.10 confidence
- Placeholder detected: skip entirely
- Sensitive variable name: +0.10 confidence

**Evidence**:
- Layered PII detection: https://www.elastic.co/observability-labs/blog/pii-ner-regex-assess-redact-part-2
- PII redaction best practices: https://synthmetric.com/pii-redaction-tactics-for-safer-datasets/

---

### R18: Generation Context with Provenance

**Priority**: P1
**Effort**: Medium

**What to Build**:
Same generation context system as v1 (pattern gatherer, tribal gatherer, constraint gatherer, anti-pattern gatherer), with these enhancements:

1. **Token budget allocation** (configurable):
   - Patterns: 30%
   - Tribal: 25%
   - Constraints: 20%
   - Anti-patterns: 15%
   - Related: 10%

2. **Provenance tracking**: Record what influenced generated code (pattern_followed, tribal_applied, constraint_enforced, antipattern_avoided).

3. **Feedback loop**: Process generation outcomes (accepted/modified/rejected). Adjust confidence of influencing memories based on outcome.

4. **Validation**: Check generated code against patterns, tribal knowledge, and anti-patterns before returning context.

5. **Provenance comments** (NEW): Generate inline code comments explaining why certain patterns were followed:
   ```
   // [drift:tribal] Always use bcrypt with 12 salt rounds for password hashing
   // [drift:pattern] auth-password-hashing (confidence: 0.92)
   ```

---

### R19: MCP Tool Layer

**Priority**: P0
**Effort**: Medium

**What to Build**:
33 MCP tools (same as v1) as thin TypeScript wrappers over the Rust Cortex engine via NAPI.

**Key tools**:
- `drift_memory_add` — Create memory with auto-deduplication (R13) and causal inference (R11)
- `drift_memory_search` — Hybrid search (FA1) with session deduplication (R16)
- `drift_why` — Full "why" context with causal narratives (R12)
- `drift_memory_learn` — Correction analysis with principle extraction (R13)
- `drift_context` — Orchestrated context retrieval (R4) with generation context (R18)

**The MCP tools stay in TypeScript**. They are thin JSON-RPC wrappers that call Rust via NAPI. No performance-critical logic in the tool layer.

---

### R20: Observability Dashboard

**Priority**: P2
**Effort**: Medium

**What to Build**:
Comprehensive health and observability for the memory system:

1. **Health report** (enhanced from v1):
   - Total memories by type
   - Average confidence by type
   - Stale memory count and trend
   - Contradiction count and resolution rate
   - Consolidation frequency and effectiveness
   - Storage size and growth rate
   - Embedding cache hit rates (L1/L2/L3)
   - Retrieval latency percentiles (p50, p95, p99)

2. **Retrieval effectiveness** (NEW):
   - Per-intent hit rate
   - Token efficiency ratio
   - Most/least useful memories
   - Query expansion effectiveness

3. **Recommendations** (NEW):
   - "5 memories need validation" (low confidence + high importance)
   - "3 contradictions unresolved" (flagged for review)
   - "Consolidation recommended" (high episodic count)
   - "Embedding cache cold" (low L1 hit rate)

---

### R21: Memory Versioning

**Priority**: P2
**Effort**: Medium

**What to Build**:
Track how memory content evolves over time, not just confidence changes.

```sql
CREATE TABLE memory_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  memory_id TEXT NOT NULL,
  version INTEGER NOT NULL,
  content TEXT NOT NULL,        -- JSON: full memory content at this version
  summary TEXT NOT NULL,
  confidence REAL NOT NULL,
  changed_by TEXT NOT NULL,     -- system|user|consolidation|learning
  change_reason TEXT,
  created_at TEXT NOT NULL,
  UNIQUE(memory_id, version)
);
```

**Use cases**:
- "How has our understanding of the auth pattern evolved?"
- "What did this memory say before the last correction?"
- "Roll back a memory to a previous version"
- Audit trail for compliance

---

### R22: Rust Crate Structure

**Priority**: P0
**Effort**: Architecture decision

**What to Build**:
Organize the Rust Cortex implementation into focused crates:

```
crates/cortex/
├── cortex-core/        # Types, traits, BaseMemory, 23 memory types, errors
├── cortex-storage/     # SQLite storage, migrations, FTS5, audit log
├── cortex-embeddings/  # ONNX provider, cache (moka), enrichment
├── cortex-retrieval/   # Hybrid search, RRF, re-ranking, intent weighting
├── cortex-causal/      # petgraph, inference, traversal, narrative
├── cortex-learning/    # Correction analysis, principle extraction, calibration
├── cortex-decay/       # Decay calculation, adaptive half-lives
├── cortex-validation/  # 4-dimension validation, healing
├── cortex-compression/ # 4-level compression, token budgeting
├── cortex-prediction/  # Signal gathering, 4 strategies, cache
├── cortex-session/     # Session management, deduplication, analytics
├── cortex-privacy/     # PII/secret sanitization (50+ patterns)
├── cortex-consolidation/ # 5-phase pipeline, adaptive scheduling
└── cortex-napi/        # NAPI bindings for TypeScript interop
```

**Key Rust crate mappings**:
| Dependency | Purpose |
|---|---|
| `rusqlite` (bundled) | SQLite storage + FTS5 |
| `ort` | ONNX Runtime for embedding inference |
| `petgraph` | Causal graph operations |
| `moka` | Concurrent caching (L1, prediction) |
| `tiktoken-rs` | Accurate token counting |
| `blake3` | Content hashing |
| `uuid` | Memory ID generation |
| `chrono` | Bitemporal time handling |
| `serde` + `serde_json` | Typed serialization |
| `thiserror` | Structured errors |
| `rayon` | Parallel batch operations |
| `tokio` | Async embedding inference |
| `regex` | Privacy sanitization |

---

## Build Order

```
Phase 0 (Architecture):  FA1 + FA2 + FA3 + R22      [Decisions before code]
Phase 1 (Storage):       R1 → R2 → R3                [Storage, Embeddings, Tokens]
Phase 2 (Retrieval):     R4 → R5 → R6                [Hybrid Search, Compression, Intents]
Phase 3 (Knowledge):     R7 → R8 → R9 → R10          [Consolidation, Decay, Contradiction, Validation]
Phase 4 (Causal):        R11 → R12                    [Causal Graph, Why System]
Phase 5 (Learning):      R13 → R14 → R15              [Corrections, Active Learning, Prediction]
Phase 6 (Integration):   R16 → R17 → R18 → R19 → R20 → R21  [Session, Privacy, Generation, MCP, Observability, Versioning]
```

Note: Phase 6 items R16-R18 should be built alongside Phases 2-5 as they provide cross-cutting concerns. Listed separately for clarity.

---

## Dependency Graph

```
FA1 (Hybrid DB) ──────→ R1 (Storage) ──→ R4 (Retrieval) ──→ R12 (Why)
FA2 (Code Embeddings) ─→ R2 (Embedding Engine) ──→ R4       │
FA3 (Errors + Audit) ──→ ALL subsystems                      ↓
R22 (Crate Structure) ─→ ALL subsystems               R18 (Generation)
                                                              │
R3 (Token Counting) ───→ R5 (Compression) ──→ R4             ↓
                                                       R19 (MCP Tools)
R1 (Storage) ──────────→ R7 (Consolidation)
                    ├───→ R8 (Decay)
                    ├───→ R9 (Contradiction) ──→ R11 (Causal Graph)
                    ├───→ R10 (Validation)
                    ├───→ R13 (Learning) ──→ R14 (Active Learning)
                    ├───→ R15 (Prediction)
                    ├───→ R16 (Session)
                    ├───→ R17 (Privacy)
                    └───→ R21 (Versioning)

R11 (Causal) ──────────→ R12 (Why System)
R4 (Retrieval) ────────→ R15 (Prediction, pre-compute)
R16 (Session) ─────────→ R20 (Observability)
```

---

## Risk Assessment

| Risk | Mitigation |
|---|---|
| ONNX model loading is slow on first run | Pre-download models during `drift setup`. Cache loaded models in L3. |
| sqlite-vec brute-force search too slow at scale | Pre-filter by type/importance. Use Matryoshka truncation for fast search. |
| Causal graph grows unbounded | Prune weak edges (strength < 0.2) and old unvalidated edges periodically. |
| LLM dependency for consolidation abstraction | Rule-based fallback for air-gapped environments (R7). |
| NAPI bridge complexity for Rust ↔ TS | Use napi-rs with typed bindings. Keep MCP tools in TS as thin wrappers. |
| Memory versioning storage growth | Limit to last 10 versions per memory. Compress old versions. |
| Hybrid search query complexity | Abstract behind a `HybridSearcher` that encapsulates the FTS5 + vec + RRF logic. |
| Token counting overhead | Cache counts per content hash (R3). Amortized cost is near-zero. |

---

## Quality Checklist

- [x] All 25 source documents in 06-cortex/ accounted for
- [x] All v2 notes from every source document addressed
- [x] All 12 limitations from RECAP resolved in recommendations
- [x] Every recommendation framed as "build new" not "migrate/port"
- [x] External evidence cited for every architectural decision
- [x] Build order defined with dependency graph
- [x] No feature deferred to "add later" — everything built into the right phase
- [x] Traceability: every source doc maps to at least one recommendation
- [x] Risk assessment with mitigations
- [x] Rust crate structure defined
- [x] NAPI boundary clearly defined (MCP tools in TS, everything else in Rust)
