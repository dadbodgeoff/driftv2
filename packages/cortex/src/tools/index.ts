/**
 * Tool registry — registers all 38 MCP tools.
 *
 * Each tool is a thin JSON-RPC wrapper over CortexClient methods.
 * Tools are grouped by domain and registered in a flat map for MCP dispatch.
 */

import type { CortexClient } from "../bridge/client.js";
import type { McpToolDefinition } from "../bridge/types.js";

// Memory (8)
import { driftMemoryAdd } from "./memory/drift_memory_add.js";
import { driftMemorySearch } from "./memory/drift_memory_search.js";
import { driftMemoryGet } from "./memory/drift_memory_get.js";
import { driftMemoryUpdate } from "./memory/drift_memory_update.js";
import { driftMemoryDelete } from "./memory/drift_memory_delete.js";
import { driftMemoryList } from "./memory/drift_memory_list.js";
import { driftMemoryLink } from "./memory/drift_memory_link.js";
import { driftMemoryUnlink } from "./memory/drift_memory_unlink.js";

// Retrieval (3)
import { driftContext } from "./retrieval/drift_context.js";
import { driftSearch } from "./retrieval/drift_search.js";
import { driftRelated } from "./retrieval/drift_related.js";

// Why (4)
import { driftWhy } from "./why/drift_why.js";
import { driftExplain } from "./why/drift_explain.js";
import { driftCounterfactual } from "./why/drift_counterfactual.js";
import { driftIntervention } from "./why/drift_intervention.js";

// Learning (3)
import { driftMemoryLearn } from "./learning/drift_memory_learn.js";
import { driftFeedback } from "./learning/drift_feedback.js";
import { driftValidate } from "./learning/drift_validate.js";

// Generation (2)
import { driftGenContext } from "./generation/drift_gen_context.js";
import { driftGenOutcome } from "./generation/drift_gen_outcome.js";

// System (8)
import { driftCortexStatus } from "./system/drift_cortex_status.js";
import { driftCortexMetrics } from "./system/drift_cortex_metrics.js";
import { driftCortexConsolidate } from "./system/drift_cortex_consolidate.js";
import { driftCortexValidate } from "./system/drift_cortex_validate.js";
import { driftCortexGc } from "./system/drift_cortex_gc.js";
import { driftCortexExport } from "./system/drift_cortex_export.js";
import { driftCortexImport } from "./system/drift_cortex_import.js";
import { driftCortexReembed } from "./system/drift_cortex_reembed.js";

// Prediction (2)
import { driftPredict } from "./prediction/drift_predict.js";
import { driftPreload } from "./prediction/drift_preload.js";

// Temporal (5)
import { driftTimeTravel } from "./temporal/drift_time_travel.js";
import { driftTimeDiff } from "./temporal/drift_time_diff.js";
import { driftTimeReplay } from "./temporal/drift_time_replay.js";
import { driftKnowledgeHealth } from "./temporal/drift_knowledge_health.js";
import { driftKnowledgeTimeline } from "./temporal/drift_knowledge_timeline.js";

/** All 38 tool factory functions. */
const TOOL_FACTORIES: ((client: CortexClient) => McpToolDefinition)[] = [
  // Memory (8)
  driftMemoryAdd,
  driftMemorySearch,
  driftMemoryGet,
  driftMemoryUpdate,
  driftMemoryDelete,
  driftMemoryList,
  driftMemoryLink,
  driftMemoryUnlink,
  // Retrieval (3)
  driftContext,
  driftSearch,
  driftRelated,
  // Why (4)
  driftWhy,
  driftExplain,
  driftCounterfactual,
  driftIntervention,
  // Learning (3)
  driftMemoryLearn,
  driftFeedback,
  driftValidate,
  // Generation (2)
  driftGenContext,
  driftGenOutcome,
  // System (8)
  driftCortexStatus,
  driftCortexMetrics,
  driftCortexConsolidate,
  driftCortexValidate,
  driftCortexGc,
  driftCortexExport,
  driftCortexImport,
  driftCortexReembed,
  // Prediction (2)
  driftPredict,
  driftPreload,
  // Temporal (5)
  driftTimeTravel,
  driftTimeDiff,
  driftTimeReplay,
  driftKnowledgeHealth,
  driftKnowledgeTimeline,
];

/** Immutable map of tool name → tool definition. */
export type ToolRegistry = ReadonlyMap<string, McpToolDefinition>;

/**
 * Register all 33 MCP tools for a given CortexClient instance.
 * Returns an immutable map of tool name → definition.
 */
export function registerTools(client: CortexClient): ToolRegistry {
  const registry = new Map<string, McpToolDefinition>();
  for (const factory of TOOL_FACTORIES) {
    const tool = factory(client);
    if (registry.has(tool.name)) {
      throw new Error(`Duplicate tool name: ${tool.name}`);
    }
    registry.set(tool.name, tool);
  }
  return registry;
}

/**
 * Get a list of all tool definitions (for MCP tool listing).
 */
export function listTools(registry: ToolRegistry): McpToolDefinition[] {
  return Array.from(registry.values());
}

/**
 * Dispatch a tool call by name.
 */
export async function callTool(
  registry: ToolRegistry,
  name: string,
  args: Record<string, unknown>,
): Promise<unknown> {
  const tool = registry.get(name);
  if (!tool) {
    throw new Error(`Unknown tool: ${name}. Available: ${Array.from(registry.keys()).join(", ")}`);
  }
  return tool.handler(args);
}
