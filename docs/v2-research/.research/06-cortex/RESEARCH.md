# 06 Cortex Memory System — External Research

> Phase 3: Verifiable best practices from trusted sources, applied to Drift's Cortex memory system.

---

## R1: Persistent AI Memory Architecture — Mem0

**Source**: https://arxiv.org/html/2504.19413
**Type**: Tier 1 (Academic paper — peer-reviewed, published on arXiv)
**Accessed**: 2026-02-06

**Source**: https://memo.d.foundation/breakdown/mem0
**Type**: Tier 2 (Industry Expert — detailed technical breakdown)
**Accessed**: 2026-02-06

**Source**: https://www.mem0.ai/blog/ai-memory-layer-guide
**Type**: Tier 2 (Industry Expert — Mem0 official engineering blog)
**Accessed**: 2026-02-06

**Key Findings**:
- Mem0 introduces a two-phase memory pipeline: (1) Extraction phase uses conversation summaries and recent messages to identify salient memories, (2) Update phase compares each candidate fact against existing memories using vector similarity, then an LLM determines whether to ADD, UPDATE, DELETE, or NOOP — ensuring consistency and avoiding redundancy.
- The Mem0g (graph) variant represents memories as a directed labeled graph where nodes are entities (with types, embeddings, metadata) and edges are relationships as triplets (source, relation, destination). This enables multi-hop reasoning that flat memory stores cannot support.
- Mem0 achieves 26% relative improvement over OpenAI's memory system on the LOCOMO benchmark, with 91% lower p95 latency and 90%+ token cost savings compared to full-context approaches.
- Implicit forgetting via relevance filtering: rather than processing entire conversation histories, Mem0 selectively extracts and retrieves only relevant information, reducing computational overhead.
- The architecture uses a pluggable backend supporting multiple vector stores (Qdrant, ChromaDB, Pinecone, FAISS) and graph databases (Neo4j).

**Applicability to Drift**:
Cortex's architecture shares many principles with Mem0 (typed memories, confidence scoring, contradiction detection) but lacks Mem0's graph-based memory representation for multi-hop reasoning. Cortex's relationship system (memory_relationships table) is a step toward this, but it's not a first-class graph with entity nodes and typed edges. Adopting a graph-based memory layer (even as an optional enhancement) would enable richer "why" queries and cross-entity reasoning. The two-phase extraction/update pipeline is also more principled than Cortex's current approach of creating memories directly — adding an explicit deduplication/update phase before storage would improve memory quality.

**Confidence**: High — peer-reviewed paper with reproducible benchmarks on established datasets.

---

## R2: Hybrid Search — Reciprocal Rank Fusion (RRF)

**Source**: https://simonwillison.net/2024/Oct/4/hybrid-full-text-search-and-vector-search-with-sqlite/
**Type**: Tier 2 (Industry Expert — Simon Willison, creator of Datasette, SQLite expert)
**Accessed**: 2026-02-06

**Source**: https://learn.microsoft.com/en-us/azure/search/hybrid-search-overview
**Type**: Tier 1 (Official documentation — Microsoft Azure)
**Accessed**: 2026-02-06

**Source**: https://www.singlestore.com/blog/hybrid-search-using-reciprocal-rank-fusion-in-sql/
**Type**: Tier 2 (Industry Expert — SingleStore engineering blog)
**Accessed**: 2026-02-06

**Key Findings**:
- Hybrid search combines full-text search (lexical/keyword matching) with vector search (semantic similarity) to produce results that benefit from both approaches. Pure vector search can miss exact keyword matches; pure full-text search misses semantic meaning.
- Reciprocal Rank Fusion (RRF) is the standard algorithm for combining results: `score = Σ 1/(k + rank_i)` where k is a smoothing constant (typically 60) and rank_i is the item's rank in each search method. RRF is simple, effective, and doesn't require score normalization across different search methods.
- SQLite supports both FTS5 (full-text search) and sqlite-vec (vector search). Combining them with RRF in a single query is achievable but requires careful SQL construction.
- Microsoft Azure AI Search uses RRF as its default fusion method for hybrid queries, validating it as an enterprise-grade approach.
- Hybrid search consistently outperforms either method alone across diverse retrieval benchmarks.

**Applicability to Drift**:
Cortex currently uses vector-only retrieval for memory search. This means a query for "bcrypt password hashing" might miss a memory that contains the exact phrase "bcrypt" but has a slightly different embedding. Adding FTS5 to cortex.db and implementing RRF fusion would improve retrieval precision significantly, especially for technical terms, function names, and specific patterns. SQLite already supports both FTS5 and sqlite-vec, so this is architecturally straightforward.

**Confidence**: High — RRF is well-established, used by Azure, Elasticsearch, and other production search systems.

---

## R3: Code Embedding Models — State of the Art

**Source**: https://modal.com/blog/6-best-code-embedding-models-compared
**Type**: Tier 2 (Industry Expert — Modal engineering blog, comprehensive comparison)
**Accessed**: 2026-02-06

**Source**: https://arxiv.org/html/2411.12644v2
**Type**: Tier 1 (Academic paper — CodeXEmbed, state-of-the-art on CoIR benchmark)
**Accessed**: 2026-02-06

**Source**: https://jina.ai/models/jina-code-embeddings-1.5b/
**Type**: Tier 1 (Official documentation — Jina AI)
**Accessed**: 2026-02-06

**Source**: https://www.qodo.ai/blog/qodo-embed-1-code-embedding-code-retrieval/
**Type**: Tier 2 (Industry Expert — Qodo engineering blog)
**Accessed**: 2026-02-06

**Key Findings**:
- Code-specific embedding models significantly outperform general-purpose models for code retrieval tasks. The word "snowflake" in a code model maps to data warehousing, not weather.
- Top models as of 2025-2026:
  - VoyageCode3: 32K context, 2048 dimensions, trained on trillions of tokens across 300+ languages. API-only.
  - Jina Code Embeddings v2/1.5b: 137M-1.5B parameters, 8192 context, Apache 2.0 license, supports 15+ programming languages. Open weights.
  - Nomic Embed Code: 7B parameters, Apache 2.0, strong cross-language performance. Open weights.
  - CodeSage Large V2: 1.3B parameters, Matryoshka representation learning (flexible dimensions), Apache 2.0. Open weights.
  - CodeRankEmbed: 137M parameters, 8192 context, MIT license, state-of-the-art code retrieval. Open weights.
- Matryoshka Representation Learning allows truncating embeddings to smaller dimensions (128, 256, 512) with minimal performance loss — useful for storage/speed tradeoffs.
- For local inference, Hugging Face's Text Embeddings Inference (Rust-based) provides higher throughput and lower latency than Python-based alternatives.

**Applicability to Drift**:
Cortex uses 384-dimensional embeddings from Transformers.js with a general-purpose model. Switching to a code-specific embedding model (Jina Code v2 or CodeRankEmbed for local, VoyageCode3 for API) would dramatically improve retrieval quality for code-related memories. The Matryoshka approach is particularly interesting — Cortex could store 1024-dim embeddings but use 384-dim for fast search and full dimensions for re-ranking. For the Rust migration, the `ort` crate (ONNX Runtime) can run these models locally with 3-5x speedup over JavaScript.

**Confidence**: High — benchmarked on established code retrieval datasets (CodeSearchNet, CoIR, MTEB).

---

## R4: Rust Embedding Inference — ort (ONNX Runtime)

**Source**: https://ort.pyke.io/
**Type**: Tier 1 (Official documentation — ort crate)
**Accessed**: 2026-02-06

**Source**: https://github.com/pykeio/ort
**Type**: Tier 1 (Official repository — 1.5K+ stars, actively maintained)
**Accessed**: 2026-02-06

**Source**: https://markaicode.com/rust-onnx-ml-models-2025/
**Type**: Tier 3 (Community validated — benchmarks and examples)
**Accessed**: 2026-02-06

**Key Findings**:
- `ort` is the Rust binding for ONNX Runtime, maintained by Microsoft. It supports CPU, CUDA, TensorRT, OpenVINO, and other execution providers.
- Rust + ONNX Runtime delivers 3-5x faster inference than Python equivalents while using 60-80% less memory (2025 benchmarks).
- ONNX Runtime supports model quantization (INT8, FP16) for further speedup with minimal accuracy loss.
- Hugging Face's Text Embeddings Inference (TEI) uses `ort` internally for production embedding serving.
- The `ort` crate supports async inference via tokio, batch processing, and dynamic input shapes — all needed for embedding generation.

**Applicability to Drift**:
For Cortex's Rust migration, `ort` is the clear choice for local embedding inference. It can load any ONNX-exported embedding model (Jina Code, CodeRankEmbed, etc.) and run inference with hardware acceleration. This replaces Transformers.js with a 3-5x speedup. The async support means embedding generation won't block the main thread during memory creation.

**Confidence**: High — `ort` is the de facto standard for Rust ML inference, backed by Microsoft's ONNX Runtime.

---

## R5: Graph Libraries in Rust — petgraph

**Source**: https://docs.rs/petgraph/
**Type**: Tier 1 (Official crate documentation)
**Accessed**: 2026-02-06

**Source**: https://depth-first.com/articles/2020/02/03/graphs-in-rust-an-introduction-to-petgraph/
**Type**: Tier 3 (Community validated — comprehensive tutorial)
**Accessed**: 2026-02-06

**Source**: https://lib.rs/crates/petgraph
**Type**: Tier 1 (Crate registry — 10M+ downloads)
**Accessed**: 2026-02-06

**Key Findings**:
- petgraph provides 4 graph implementations: `Graph` (general purpose), `StableGraph` (stable indices after removal), `GraphMap` (hashable node IDs), `MatrixGraph` (adjacency matrix).
- Built-in algorithms: DFS, BFS, Dijkstra, Bellman-Ford, A*, Tarjan's SCC, topological sort, minimum spanning tree, isomorphism.
- Traversals are implemented as Rust iterators — composable and zero-cost.
- `StableGraph` is ideal for graphs where nodes/edges are frequently added and removed (like Cortex's causal graph where edges are created, validated, and pruned).
- DOT format export for visualization with Graphviz.
- Supports both directed and undirected graphs with arbitrary node and edge data.

**Applicability to Drift**:
Cortex's causal system currently stores edges in SQLite and traverses by repeated queries. For the Rust migration, maintaining an in-memory `StableGraph` (synced with SQLite) would dramatically speed up causal traversal, narrative generation, and contradiction propagation. `StableGraph` is the right choice because causal edges are frequently created and pruned. The built-in Tarjan's SCC can detect circular causal chains. DFS/BFS iterators map directly to Cortex's traceOrigins/traceEffects operations.

**Confidence**: High — petgraph is the standard Rust graph library with 10M+ downloads and use in production systems including Fuchsia OS.

---

## R6: Concurrent Caching in Rust — moka

**Source**: https://docs.rs/moka/latest/moka/
**Type**: Tier 1 (Official crate documentation)
**Accessed**: 2026-02-06

**Source**: https://github.com/moka-rs/moka
**Type**: Tier 1 (Official repository — 2K+ stars)
**Accessed**: 2026-02-06

**Key Findings**:
- Moka is a high-performance concurrent cache inspired by Java's Caffeine library.
- Uses TinyLFU admission policy + LRU eviction — near-optimal hit ratio.
- Thread-safe with full concurrency for reads and high concurrency for writes.
- Supports: max capacity (by count or weighted size), per-entry TTL, time-to-idle, async cache variant, entry listeners (on eviction/insertion).
- Sync and async variants available.
- Size-aware eviction: entries can have custom weights, enabling memory-bounded caches.

**Applicability to Drift**:
Cortex's L1 cache (in-process Map) should be replaced with `moka::sync::Cache` (or `moka::future::Cache` for async). Benefits: (1) TinyLFU provides better hit ratio than simple LRU, (2) per-entry TTL enables adaptive expiration (prediction cache can use short TTL, embedding cache can use long TTL), (3) size-aware eviction prevents memory bloat from large embeddings, (4) thread-safe without external locking. For the prediction cache specifically, moka's TTL support replaces the custom 5-minute TTL implementation.

**Confidence**: High — moka is the most popular Rust caching library, inspired by the battle-tested Caffeine.

---

## R7: Memory Consolidation — Neuroscience-Inspired Approaches

**Source**: https://arxiv.org/html/2503.18371
**Type**: Tier 1 (Academic paper — continual learning with spaced repetition)
**Accessed**: 2026-02-06

**Source**: https://link.springer.com/chapter/10.1007%2F978-3-030-52240-7_65
**Type**: Tier 1 (Academic — Springer, adaptive forgetting curves for spaced repetition)
**Accessed**: 2026-02-06

**Source**: https://arxiv.org/html/2506.12034v2
**Type**: Tier 1 (Academic paper — human-like forgetting curves in neural networks)
**Accessed**: 2026-02-06

**Key Findings**:
- Ebbinghaus's forgetting curve shows ~50% information loss within 1 hour, ~70% within 24 hours, ~90% within a week without reinforcement. Spaced repetition counteracts this by reviewing at increasing intervals.
- The "Task-Focused Consolidation with Spaced Recall" (TFC-SR) approach enhances experience replay with an Active Recall Probe mechanism — periodically testing whether knowledge is still accessible before deciding to consolidate or discard.
- Adaptive forgetting curves model per-item decay rates rather than using a single global curve. Items that are harder to remember get shorter review intervals.
- The key insight for AI memory systems: consolidation should be triggered not just by time, but by retrieval difficulty. If a memory is hard to retrieve (low similarity scores when it should match), it needs reinforcement or consolidation.

**Applicability to Drift**:
Cortex's consolidation engine uses time-based triggers (age > 7 days). Adding retrieval-difficulty-based triggers would improve consolidation quality: if a memory that should be relevant keeps scoring low in retrieval, it may need to be consolidated with supporting context or its embedding refreshed. The adaptive forgetting curve concept maps to Cortex's type-specific half-lives, but could be extended to per-memory adaptive decay rates based on access patterns. The Active Recall Probe concept maps to Cortex's active learning loop — periodically testing whether memories are still valid and useful.

**Confidence**: High — grounded in established neuroscience (Ebbinghaus, 1885) with modern computational validation.

---

## R8: SQLite Vector Search — sqlite-vec Best Practices

**Source**: https://github.com/asg017/sqlite-vec
**Type**: Tier 1 (Official repository — by Alex Garcia, SQLite extension author)
**Accessed**: 2026-02-06

**Source**: https://stephencollins.tech/posts/how-to-use-sqlite-vec-to-store-and-query-vector-embeddings
**Type**: Tier 3 (Community validated — practical guide)
**Accessed**: 2026-02-06

**Source**: https://marcobambini.substack.com/p/the-state-of-vector-search-in-sqlite
**Type**: Tier 2 (Industry Expert — comprehensive comparison of SQLite vector extensions)
**Accessed**: 2026-02-06

**Key Findings**:
- sqlite-vec is a clean C implementation exposing brute-force KNN search via virtual tables. It supports multiple distance metrics (cosine, L2, inner product) and SIMD acceleration.
- Vectors must live in separate virtual tables, making queries more complex (JOIN required).
- For datasets under ~100K vectors, brute-force search is fast enough (sub-millisecond for 384-dim). For larger datasets, consider approximate nearest neighbor (ANN) indexes.
- Pre-formatting text before generating embeddings improves semantic relevance — include context like function signatures, file paths, and category labels.
- sqlite-vec is the successor to sqlite-vss (which was based on Faiss). sqlite-vec is more portable and easier to install.
- Hybrid search combining FTS5 + sqlite-vec with RRF is achievable in SQLite but requires careful query construction.

**Applicability to Drift**:
Cortex uses sqlite-vec with 384-dim vectors. For typical project sizes (hundreds to low thousands of memories), brute-force search is adequate. For enterprise scale (10K+ memories), consider: (1) dimensionality reduction via Matryoshka embeddings for faster search, (2) pre-filtering by type/importance before vector search to reduce candidate set, (3) embedding enrichment — prepend memory type and category to text before embedding for better semantic separation. The hybrid FTS5 + sqlite-vec approach (R2) should be implemented at this layer.

**Confidence**: High — sqlite-vec is the standard SQLite vector extension, actively maintained by its creator.

---

## R9: PII Detection — Layered Approach

**Source**: https://www.elastic.co/observability-labs/blog/pii-ner-regex-assess-redact-part-2
**Type**: Tier 2 (Industry Expert — Elastic engineering blog)
**Accessed**: 2026-02-06

**Source**: https://synthmetric.com/pii-redaction-tactics-for-safer-datasets/
**Type**: Tier 3 (Community validated — practical PII redaction guide)
**Accessed**: 2026-02-06

**Source**: https://www.protecto.ai/blog/why-regex-fails-pii-detection-in-unstructured-text/
**Type**: Tier 2 (Industry Expert — Protecto AI, PII detection specialists)
**Accessed**: 2026-02-06

**Key Findings**:
- Regex-only PII detection breaks down with unstructured text. Real-world conversations, notes, and documents require context-aware detection.
- Best practice is a layered approach: (1) Rule-based/regex for structured patterns (emails, SSNs, credit cards), (2) NER (Named Entity Recognition) for unstructured PII (names, addresses, organizations), (3) ML-based classification for ambiguous cases.
- Precision/recall metrics should be tracked: high precision (few false positives) is critical for code memories where over-redaction destroys useful information.
- Common missed patterns in code contexts: connection strings with embedded credentials, base64-encoded secrets, environment variable values in logs, hardcoded IPs in configuration.
- Validation should include targeted QA sampling — periodically check that sanitization isn't destroying useful information.

**Applicability to Drift**:
Cortex's privacy system has only 10 regex patterns. For enterprise use, this needs significant expansion: (1) Add 50+ provider-specific secret patterns (matching the secret detection in Rust core), (2) Add connection string parsing (PostgreSQL, MySQL, MongoDB, Redis URLs with embedded passwords), (3) Add base64 detection for encoded secrets, (4) Consider NER for unstructured PII in tribal knowledge memories (names, addresses mentioned in context). However, for code-focused memories, regex + structured pattern matching is usually sufficient — NER is more important for conversation and meeting memories.

**Confidence**: Medium-High — layered approach is well-established, but the right balance depends on memory content types.

---

## R10: RAG Production Best Practices

**Source**: https://hyperion-consulting.io/en/insights/rag-optimization-production-2026-best-practices
**Type**: Tier 2 (Industry Expert — production RAG optimization guide)
**Accessed**: 2026-02-06

**Source**: https://greenlogic.eu/blog/rag-in-production-how-to-design-deploy-and-maintain-enterprise-grade-retrieval-systems/
**Type**: Tier 2 (Industry Expert — enterprise RAG design patterns)
**Accessed**: 2026-02-06

**Source**: https://iterathon.tech/blog/rag-systems-production-guide-2025
**Type**: Tier 2 (Industry Expert — RAG production guide 2026)
**Accessed**: 2026-02-06

**Key Findings**:
- Chunk size has massive impact on retrieval quality. Smaller chunks (100-256 tokens) match queries more precisely but lose context. Larger chunks (1024+ tokens) preserve context but dilute relevance. The optimal approach is hierarchical chunking with parent-child relationships.
- Re-ranking after initial retrieval significantly improves precision. A two-stage pipeline (fast retrieval → precise re-ranking) is the production standard.
- Query expansion/rewriting improves recall: rephrase the user's query into multiple variants before searching.
- Metadata filtering before vector search reduces the candidate set and improves both speed and relevance.
- Evaluation metrics: faithfulness (is the answer grounded in retrieved context?), relevance (are retrieved documents relevant?), answer correctness.
- Observability is critical: track retrieval latency, hit rates, token usage, and user feedback to continuously improve.

**Applicability to Drift**:
Cortex's retrieval engine already implements some of these patterns (intent-based filtering, token budgeting, compression). Key gaps: (1) No re-ranking stage — retrieved memories are scored once and returned. Adding a lightweight re-ranker (cross-encoder or LLM-based) would improve precision. (2) No query expansion — the focus string is used as-is. Generating 2-3 query variants would improve recall. (3) No retrieval observability — no tracking of which memories were useful vs ignored by the AI. The feedback loop exists but isn't connected to retrieval quality metrics.

**Confidence**: High — these are established production patterns used by major RAG deployments.

---

## R11: Causal Knowledge Graphs

**Source**: https://www.researchgate.net/publication/357765711_CausalKG_Causal_Knowledge_Graph_Explainability_using_interventional_and_counterfactual_reasoning
**Type**: Tier 1 (Academic paper — peer-reviewed)
**Accessed**: 2026-02-06

**Source**: https://www.preprints.org/manuscript/202512.2718
**Type**: Tier 1 (Academic preprint — causal reasoning over knowledge graphs)
**Accessed**: 2026-02-06

**Key Findings**:
- Causal Knowledge Graphs (CausalKG) combine knowledge graph structure with causal reasoning, enabling interventional ("what if we change X?") and counterfactual ("what would have happened if X hadn't occurred?") queries.
- Directed Acyclic Graphs (DAGs) are the standard representation for causal relationships. Cycles in causal graphs indicate modeling errors or feedback loops that need special handling.
- Evidence-linked and versioned knowledge units enable auditable reasoning traces — every conclusion can be traced back to its supporting evidence.
- The combination of knowledge representation with causal relationships creates an interpretable decision objective function with logical traceability.
- LLMs can assist in causal discovery by identifying causal relationships from text, achieving state-of-the-art performance on causal benchmarks.

**Applicability to Drift**:
Cortex's causal system already implements directed causal graphs with evidence tracking. Key improvements: (1) Enforce DAG constraint — detect and handle cycles (currently no cycle detection in causal graph). (2) Add counterfactual queries: "What would have happened if we hadn't adopted this pattern?" (3) Add intervention queries: "If we change this convention, what memories become invalid?" (4) Version causal edges so the evolution of causal understanding can be traced. (5) Consider using LLM-assisted causal discovery for the inference phase — the current heuristic strategies could be augmented with LLM-based relationship extraction.

**Confidence**: Medium-High — academic foundations are strong, but practical implementation in code memory systems is novel.

---

## R12: Token Counting Accuracy

**Source**: https://github.com/openai/tiktoken
**Type**: Tier 1 (Official repository — OpenAI's tokenizer, 15K+ stars)
**Accessed**: 2026-02-06

**Source**: https://docs.rs/tiktoken-rs/
**Type**: Tier 1 (Official crate documentation — Rust port of tiktoken)
**Accessed**: 2026-02-06

**Key Findings**:
- Token counting from string length is inaccurate. English text averages ~4 characters per token, but code can vary from 2-6 characters per token depending on language and content.
- tiktoken provides exact token counts for OpenAI models. The `tiktoken-rs` crate provides the same functionality in Rust.
- For non-OpenAI models (Anthropic, local models), tokenizers differ. The `tokenizers` crate (by Hugging Face) supports model-specific tokenization.
- Accurate token counting is critical for budget management — overestimation wastes context window, underestimation causes truncation.

**Applicability to Drift**:
Cortex's token estimation uses string length approximation (`utils/tokens.ts`). For enterprise-grade budget management, this should be replaced with actual tokenizer-based counting. The `tiktoken-rs` crate provides this for the Rust migration. For the TypeScript layer, `tiktoken` (npm package) or `js-tiktoken` provides accurate counts. Consider caching token counts per memory (they don't change unless content changes) to avoid repeated tokenization.

**Confidence**: High — tiktoken is the standard tokenizer, used by OpenAI and widely adopted.

---

## R13: Memory System Observability

**Source**: https://www.salesforce.com/blog/system-level-ai/
**Type**: Tier 2 (Industry Expert — Salesforce engineering blog on system-level AI)
**Accessed**: 2026-02-06

**Source**: https://greenlogic.eu/blog/rag-in-production-how-to-design-deploy-and-maintain-enterprise-grade-retrieval-systems/
**Type**: Tier 2 (Industry Expert — enterprise RAG maintenance)
**Accessed**: 2026-02-06

**Key Findings**:
- Production AI memory systems require continuous monitoring of: memory quality (average confidence, stale count, contradiction rate), retrieval effectiveness (hit rate, relevance scores, token efficiency), and system health (storage size, embedding latency, consolidation frequency).
- Memory architectures that enable continuity need reasoning modules that handle complex logic, simulation environments for continuous improvement, and orchestration layers that coordinate it all.
- Feedback loops between retrieval quality and memory management are essential — if retrieved memories are consistently ignored by the AI, they should be flagged for review or confidence reduction.
- Audit trails for memory operations enable debugging and compliance.

**Applicability to Drift**:
Cortex has a `getHealth()` method that reports basic stats (total memories, average confidence, stale count). For enterprise grade, this needs: (1) Retrieval effectiveness tracking — was the retrieved memory actually used by the AI? (2) Token efficiency metrics — how much of the budget was useful vs wasted? (3) Memory quality trends over time — is the system getting smarter or degrading? (4) Audit trail for all memory mutations (create, update, archive, confidence changes). The `memory_usage_history` and `token_usage_snapshots` tables exist but need richer instrumentation.

**Confidence**: Medium-High — observability principles are well-established, but specific metrics for AI memory systems are still evolving.

---

## R14: Embedding Enrichment for Improved Retrieval

**Source**: https://stephencollins.tech/posts/how-to-use-sqlite-vec-to-store-and-query-vector-embeddings
**Type**: Tier 3 (Community validated — practical embedding guide)
**Accessed**: 2026-02-06

**Source**: https://hyperion-consulting.io/en/insights/rag-optimization-production-2026-best-practices
**Type**: Tier 2 (Industry Expert — RAG optimization)
**Accessed**: 2026-02-06

**Key Findings**:
- Pre-formatting text before embedding generation significantly improves retrieval quality. Including metadata context (category, type, domain) in the embedded text helps the model create more discriminative embeddings.
- For code-related content, including the programming language, framework, and file path in the embedding text improves cross-language retrieval.
- Hypothetical Document Embeddings (HyDE): instead of embedding the query directly, generate a hypothetical answer and embed that. This bridges the gap between query style and document style.

**Applicability to Drift**:
Cortex currently embeds the memory's text content directly. Enriching the embedded text with structured metadata would improve retrieval:
```
[tribal|critical|security] Never call the payment API without idempotency keys.
Files: src/payments/api.ts, src/checkout/service.ts
Patterns: payment-api-pattern, idempotency-pattern
```
This gives the embedding model more signal about what the memory is about, improving similarity search for related queries. The enrichment should be applied at embedding time (not query time) so it's a one-time cost.

**Confidence**: Medium — the principle is well-established in RAG literature, but the specific enrichment format for code memories needs experimentation.

---

## R15: Governed Memory Fabric — Epistemic Identity

**Source**: https://www.csharp.com/article/the-gdel-autonomous-memory-fabric-db-layer-the-database-substrate-that-makes-c/
**Type**: Tier 3 (Community validated — detailed architectural analysis)
**Accessed**: 2026-02-06

**Key Findings**:
- A governed memory substrate treats memory like regulated infrastructure: every write is gated, every memory item carries epistemic identity (provenance, confidence, evidence chain), every promoted knowledge unit is evidence-linked and versioned.
- Retrieval should be policy-aware and trust-weighted — not all memories are equally trustworthy, and retrieval should account for this.
- Reasoning should be replayable as a formal, auditable execution trace.
- Memory promotion (from raw observation to trusted knowledge) should require evidence thresholds — not just time-based consolidation.

**Applicability to Drift**:
Cortex's consolidation currently promotes episodic memories to semantic based on time and frequency. Adding evidence-based promotion thresholds would improve knowledge quality: a memory should only be promoted to semantic if it has been confirmed by multiple episodes, validated by user feedback, or supported by pattern data. The concept of "epistemic identity" (every memory knows where it came from and why it's trusted) maps to Cortex's provenance tracking in the generation system — but should be extended to all memory operations, not just generation.

**Confidence**: Medium — the architectural principles are sound, but the specific implementation is novel and untested at scale.

