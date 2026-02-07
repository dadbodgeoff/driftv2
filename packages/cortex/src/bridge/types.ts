/**
 * TypeScript type definitions for the Cortex NAPI bridge.
 *
 * GENERATED TYPES: All Rust-mirroring types below are validated against ts-rs
 * generated bindings from `cortex-core` Rust structs/enums with `#[derive(TS)]`.
 *
 * To regenerate: `cargo test -p cortex-core export_bindings`
 * Generated files: `crates/cortex/cortex-core/bindings/*.ts`
 *
 * CI enforces sync: `cargo test export_bindings -p cortex-core && git diff --exit-code`
 *
 * The types below are kept in sync with Rust automatically. If a Rust struct
 * changes, `cargo test` regenerates the .ts file and CI catches the diff.
 *
 * TS-only types (error codes, MCP tool definitions) remain at the bottom.
 */

// ─── Enums ───────────────────────────────────────────────────────────────────

/** 23 memory type variants across 3 categories. Generated from Rust `MemoryType`. */
export type MemoryType =
  // Domain-agnostic (9)
  | "core"
  | "tribal"
  | "procedural"
  | "semantic"
  | "episodic"
  | "decision"
  | "insight"
  | "reference"
  | "preference"
  // Code-specific (4)
  | "pattern_rationale"
  | "constraint_override"
  | "decision_context"
  | "code_smell"
  // Universal V2 (10)
  | "agent_spawn"
  | "entity"
  | "goal"
  | "feedback"
  | "workflow"
  | "conversation"
  | "incident"
  | "meeting"
  | "skill"
  | "environment";

export type Importance = "low" | "normal" | "high" | "critical";

export type HealthStatus = "healthy" | "degraded" | "unhealthy";

export type HealingActionType =
  | "confidence_adjust"
  | "citation_update"
  | "embedding_refresh"
  | "archival"
  | "human_review_flag";

export type RelationshipType =
  | "supersedes"
  | "supports"
  | "contradicts"
  | "related"
  | "derived_from"
  | "owns"
  | "affects"
  | "blocks"
  | "requires"
  | "references"
  | "learned_from"
  | "assigned_to"
  | "depends_on";

/** 18 intent types across 3 categories. Generated from Rust `Intent`. */
export type Intent =
  | "create"
  | "investigate"
  | "decide"
  | "recall"
  | "learn"
  | "summarize"
  | "compare"
  | "add_feature"
  | "fix_bug"
  | "refactor"
  | "security_audit"
  | "understand_code"
  | "add_test"
  | "review_code"
  | "deploy_migrate"
  | "spawn_agent"
  | "execute_workflow"
  | "track_progress";

// ─── Link Types ──────────────────────────────────────────────────────────────

export interface PatternLink {
  pattern_id: string;
  pattern_name: string;
}

export interface ConstraintLink {
  constraint_id: string;
  constraint_name: string;
}

export interface FileLink {
  file_path: string;
  line_start: number | null;
  line_end: number | null;
  content_hash: string | null;
}

export interface FunctionLink {
  function_name: string;
  file_path: string;
  signature: string | null;
}

// ─── Typed Content Variants ──────────────────────────────────────────────────

// Domain-agnostic

export interface CoreContent {
  project_name: string;
  description: string;
  metadata: unknown;
}

export interface TribalContent {
  knowledge: string;
  severity: string;
  warnings: string[];
  consequences: string[];
}

export interface ProceduralStep {
  order: number;
  instruction: string;
  completed: boolean;
}

export interface ProceduralContent {
  title: string;
  steps: ProceduralStep[];
  prerequisites: string[];
}

export interface SemanticContent {
  knowledge: string;
  source_episodes: string[];
  consolidation_confidence: number;
}

export interface EpisodicContent {
  interaction: string;
  context: string;
  outcome: string | null;
}

export interface Alternative {
  description: string;
  reason_rejected: string;
}

export interface DecisionContent {
  decision: string;
  rationale: string;
  alternatives: Alternative[];
}

export interface InsightContent {
  observation: string;
  evidence: string[];
}

export interface ReferenceContent {
  title: string;
  url: string | null;
  citation: string;
}

export interface PreferenceContent {
  preference: string;
  scope: string;
  value: unknown;
}

// Code-specific

export interface PatternRationaleContent {
  pattern_name: string;
  rationale: string;
  business_context: string;
  examples: string[];
}

export interface ConstraintOverrideContent {
  constraint_name: string;
  override_reason: string;
  approved_by: string;
  scope: string;
  expiry: string | null;
}

export interface DecisionContextContent {
  decision: string;
  context: string;
  adr_link: string | null;
  trade_offs: string[];
}

export interface CodeSmellContent {
  smell_name: string;
  description: string;
  bad_example: string;
  good_example: string;
  severity: string;
}


// Universal V2

export interface AgentSpawnContent {
  agent_name: string;
  configuration: unknown;
  purpose: string;
}

export interface EntityContent {
  entity_name: string;
  entity_type: string;
  description: string;
  attributes: unknown;
}

export interface GoalContent {
  title: string;
  description: string;
  progress: number;
  milestones: string[];
}

export interface FeedbackContent {
  feedback: string;
  category: string;
  source: string;
}

export interface WorkflowStep {
  order: number;
  action: string;
  condition: string | null;
}

export interface WorkflowContent {
  name: string;
  steps: WorkflowStep[];
  trigger: string | null;
}

export interface ConversationContent {
  summary: string;
  participants: string[];
  key_points: string[];
}

export interface IncidentContent {
  title: string;
  root_cause: string;
  impact: string;
  resolution: string;
  lessons_learned: string[];
}

export interface MeetingContent {
  title: string;
  attendees: string[];
  notes: string;
  action_items: string[];
}

export interface SkillContent {
  skill_name: string;
  proficiency: string;
  domain: string;
  evidence: string[];
}

export interface EnvironmentContent {
  name: string;
  config: unknown;
  platform: string | null;
}

/**
 * Tagged union for TypedContent — matches Rust's #[serde(tag = "type", content = "data")].
 * Each variant is { type: "variant_name", data: VariantContent }.
 */
export type TypedContent =
  | { type: "core"; data: CoreContent }
  | { type: "tribal"; data: TribalContent }
  | { type: "procedural"; data: ProceduralContent }
  | { type: "semantic"; data: SemanticContent }
  | { type: "episodic"; data: EpisodicContent }
  | { type: "decision"; data: DecisionContent }
  | { type: "insight"; data: InsightContent }
  | { type: "reference"; data: ReferenceContent }
  | { type: "preference"; data: PreferenceContent }
  | { type: "pattern_rationale"; data: PatternRationaleContent }
  | { type: "constraint_override"; data: ConstraintOverrideContent }
  | { type: "decision_context"; data: DecisionContextContent }
  | { type: "code_smell"; data: CodeSmellContent }
  | { type: "agent_spawn"; data: AgentSpawnContent }
  | { type: "entity"; data: EntityContent }
  | { type: "goal"; data: GoalContent }
  | { type: "feedback"; data: FeedbackContent }
  | { type: "workflow"; data: WorkflowContent }
  | { type: "conversation"; data: ConversationContent }
  | { type: "incident"; data: IncidentContent }
  | { type: "meeting"; data: MeetingContent }
  | { type: "skill"; data: SkillContent }
  | { type: "environment"; data: EnvironmentContent };

// ─── BaseMemory ──────────────────────────────────────────────────────────────

/** The universal memory struct. Every memory in the system is a BaseMemory. */
export interface BaseMemory {
  id: string;
  memory_type: MemoryType;
  content: TypedContent;
  summary: string;
  transaction_time: string;
  valid_time: string;
  valid_until: string | null;
  confidence: number;
  importance: Importance;
  last_accessed: string;
  access_count: number;
  linked_patterns: PatternLink[];
  linked_constraints: ConstraintLink[];
  linked_files: FileLink[];
  linked_functions: FunctionLink[];
  tags: string[];
  archived: boolean;
  superseded_by: string | null;
  supersedes: string | null;
  content_hash: string;
}

// ─── Retrieval & Compression ─────────────────────────────────────────────────

export interface RetrievalContext {
  focus: string;
  intent: Intent | null;
  active_files: string[];
  budget: number;
  sent_ids: string[];
}

export interface CompressedMemory {
  memory_id: string;
  memory_type: MemoryType;
  importance: Importance;
  level: number;
  text: string;
  token_count: number;
  relevance_score: number;
}

// ─── Causal ──────────────────────────────────────────────────────────────────

export interface NarrativeSection {
  title: string;
  content: string;
  memory_ids: string[];
}

export interface CausalNarrative {
  sections: NarrativeSection[];
  summary: string;
  confidence: number;
}

export interface TraversalNode {
  memory_id: string;
  depth: number;
  path_strength: number;
}

export interface TraversalResult {
  origin_id: string;
  max_depth_reached: number;
  nodes: TraversalNode[];
}

export interface InferenceResult {
  source_id: string;
  target_id: string;
  strength: number;
  suggested_relation: string;
  above_threshold: boolean;
}

// ─── Consolidation ───────────────────────────────────────────────────────────

export interface ConsolidationMetrics {
  precision: number;
  compression_ratio: number;
  lift: number;
  stability: number;
}

export interface ConsolidationResult {
  created: string[];
  archived: string[];
  metrics: ConsolidationMetrics;
}

export interface ConsolidationStatus {
  is_running: boolean;
}

export interface ConsolidationDashboard {
  total_runs: number;
  successful_runs: number;
  success_rate: number;
  is_running: boolean;
}

// ─── Learning ────────────────────────────────────────────────────────────────

export interface LearningResult {
  category: string;
  principle: string | null;
  memory_created: string | null;
}

// ─── Validation ──────────────────────────────────────────────────────────────

export interface DimensionScores {
  citation: number;
  temporal: number;
  contradiction: number;
  pattern_alignment: number;
}

export interface HealingAction {
  action_type: HealingActionType;
  description: string;
  applied: boolean;
}

export interface ValidationResult {
  memory_id: string;
  dimension_scores: DimensionScores;
  overall_score: number;
  healing_actions: HealingAction[];
  passed: boolean;
}

// ─── Health ──────────────────────────────────────────────────────────────────

export interface SubsystemHealth {
  name: string;
  status: HealthStatus;
  message: string | null;
}

export interface HealthMetrics {
  total_memories: number;
  active_memories: number;
  archived_memories: number;
  average_confidence: number;
  db_size_bytes: number;
  embedding_cache_hit_rate: number;
}

export interface HealthReport {
  overall_status: HealthStatus;
  subsystems: SubsystemHealth[];
  metrics: HealthMetrics;
}

// ─── Session ─────────────────────────────────────────────────────────────────

export interface SessionContext {
  session_id: string;
  sent_memory_ids: string[];
  tokens_used: number;
  token_budget: number;
}

export interface SessionAnalytics {
  session_id: string;
  created_at: string;
  last_activity: string;
  loaded_memories_count: number;
  loaded_patterns_count: number;
  loaded_files_count: number;
  tokens_sent: number;
  queries_made: number;
}

// ─── Prediction ──────────────────────────────────────────────────────────────

export interface PredictionResult {
  memory_ids: string[];
  signals: string[];
  confidence: number;
}

export interface PreloadResult {
  preloaded_count: number;
  memory_ids: string[];
  confidence: number;
}

export interface CacheStats {
  entry_count: number;
  hits: number;
  misses: number;
  hit_rate: number;
}

// ─── Generation ──────────────────────────────────────────────────────────────

export interface BudgetAllocation {
  category: string;
  percentage: number;
  memories: CompressedMemory[];
  tokens_used: number;
}

export interface GenerationContext {
  allocations: BudgetAllocation[];
  total_tokens: number;
  total_budget: number;
}

// ─── Privacy ─────────────────────────────────────────────────────────────────

export interface SanitizeResult {
  text: string;
  redactions: number;
}

export interface PatternFailure {
  pattern_name: string;
  category: string;
  error: string;
}

export interface PatternStats {
  failure_count: number;
  has_failures: boolean;
  failures: PatternFailure[];
}

// ─── Cloud ───────────────────────────────────────────────────────────────────

export interface SyncResult {
  status: string;
  pushed: number;
  pulled: number;
  conflicts_resolved: number;
  manual_conflicts: number;
}

export interface CloudStatus {
  status: string;
  is_online: boolean;
  offline_queue_length: number;
}

// ─── Degradation ─────────────────────────────────────────────────────────────

export interface DegradationEvent {
  component: string;
  failure: string;
  fallback_used: string;
  timestamp: string;
}

// ─── Relationships ───────────────────────────────────────────────────────────

export interface RelationshipEdge {
  source_id: string;
  target_id: string;
  relationship_type: RelationshipType;
  strength: number;
  evidence: string[];
}

// ─── WhyContext ──────────────────────────────────────────────────────────────

export interface WhyEntry {
  memory_id: string;
  summary: string;
  confidence: number;
}

export interface WhyContext {
  patterns: WhyEntry[];
  decisions: WhyEntry[];
  tribal: WhyEntry[];
  warnings: string[];
}

// ─── TS-Only Types (not generated from Rust) ─────────────────────────────────

export const CortexErrorCode = {
  MEMORY_NOT_FOUND: "MEMORY_NOT_FOUND",
  INVALID_TYPE: "INVALID_TYPE",
  EMBEDDING_ERROR: "EMBEDDING_ERROR",
  STORAGE_ERROR: "STORAGE_ERROR",
  CAUSAL_CYCLE: "CAUSAL_CYCLE",
  TOKEN_BUDGET_EXCEEDED: "TOKEN_BUDGET_EXCEEDED",
  MIGRATION_ERROR: "MIGRATION_ERROR",
  SANITIZATION_ERROR: "SANITIZATION_ERROR",
  CONSOLIDATION_ERROR: "CONSOLIDATION_ERROR",
  VALIDATION_ERROR: "VALIDATION_ERROR",
  SERIALIZATION_ERROR: "SERIALIZATION_ERROR",
  CONCURRENCY_ERROR: "CONCURRENCY_ERROR",
  CLOUD_SYNC_ERROR: "CLOUD_SYNC_ERROR",
  CONFIG_ERROR: "CONFIG_ERROR",
  DEGRADED_MODE: "DEGRADED_MODE",
  RUNTIME_NOT_INITIALIZED: "RUNTIME_NOT_INITIALIZED",
} as const;

export type CortexErrorCodeType = (typeof CortexErrorCode)[keyof typeof CortexErrorCode];

// ─── MCP Tool Definition ─────────────────────────────────────────────────────

export interface McpToolDefinition {
  name: string;
  description: string;
  inputSchema: {
    type: "object";
    properties: Record<string, unknown>;
    required?: string[];
  };
  handler: (args: Record<string, unknown>) => Promise<unknown>;
}
