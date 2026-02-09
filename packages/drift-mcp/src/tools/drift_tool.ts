/**
 * drift_tool — dynamic dispatch for ~49 internal tools.
 *
 * Progressive disclosure: AI agent sees 3-4 tools initially, discovers
 * more via drift_tool. This reduces token overhead ~81% compared to
 * registering all ~52 tools individually.
 *
 * The AI agent calls drift_tool with a tool name and parameters,
 * and this handler routes to the correct NAPI function.
 */

import { loadNapi } from '../napi.js';
import type { DriftToolParams, InternalTool } from '../types.js';

/** JSON Schema for drift_tool parameters. */
export const DRIFT_TOOL_SCHEMA = {
  type: 'object' as const,
  properties: {
    tool: {
      type: 'string',
      description: 'Internal tool name (use drift_status to discover available tools)',
    },
    params: {
      type: 'object',
      description: 'Tool-specific parameters',
      additionalProperties: true,
    },
  },
  required: ['tool'],
  additionalProperties: false,
};

/** Build the internal tool catalog with NAPI-backed handlers. */
export function buildToolCatalog(): Map<string, InternalTool> {
  const catalog = new Map<string, InternalTool>();

  // Discovery tools
  register(catalog, {
    name: 'drift_status',
    description: 'Health snapshot (patterns, violations, storage)',
    category: 'discovery',
    estimatedTokens: '~200',
    handler: async () => loadNapi().drift_status(),
  });
  register(catalog, {
    name: 'drift_capabilities',
    description: 'Full tool listing with descriptions',
    category: 'discovery',
    estimatedTokens: '~500',
    handler: async () => listCapabilities(catalog),
  });

  // Surgical tools
  register(catalog, {
    name: 'drift_callers',
    description: 'Who calls this function',
    category: 'surgical',
    estimatedTokens: '~200-500',
    handler: async (p) => loadNapi().drift_call_graph(p.path as string),
  });
  register(catalog, {
    name: 'drift_reachability',
    description: 'Data flow reachability from a function',
    category: 'surgical',
    estimatedTokens: '~1000-3000',
    handler: async (p) => loadNapi().drift_reachability(p.functionId as string),
  });
  register(catalog, {
    name: 'drift_prevalidate',
    description: 'Quick pre-write validation',
    category: 'surgical',
    estimatedTokens: '~300-800',
    handler: async (p) => loadNapi().drift_check(p.path as string),
  });
  register(catalog, {
    name: 'drift_similar',
    description: 'Find similar code patterns',
    category: 'surgical',
    estimatedTokens: '~500-1500',
    handler: async (p) => loadNapi().drift_patterns(p.path as string),
  });

  // Exploration tools
  register(catalog, {
    name: 'drift_patterns_list',
    description: 'List patterns with filters + pagination',
    category: 'exploration',
    estimatedTokens: '~500-1500',
    handler: async (p) => loadNapi().drift_patterns(p.path as string ?? '.'),
  });
  register(catalog, {
    name: 'drift_security_summary',
    description: 'Security posture overview',
    category: 'exploration',
    estimatedTokens: '~800-2000',
    handler: async (p) => loadNapi().drift_check(p.path as string ?? '.', 'security'),
  });
  register(catalog, {
    name: 'drift_trends',
    description: 'Pattern trends over time',
    category: 'exploration',
    estimatedTokens: '~500-1500',
    handler: async (p) => loadNapi().drift_audit(p.path as string ?? '.'),
  });

  // Detail tools
  register(catalog, {
    name: 'drift_impact_analysis',
    description: 'Change blast radius analysis',
    category: 'detail',
    estimatedTokens: '~1000-3000',
    handler: async (p) => loadNapi().drift_impact(p.functionId as string),
  });
  register(catalog, {
    name: 'drift_taint',
    description: 'Taint flow analysis (source → sink)',
    category: 'detail',
    estimatedTokens: '~1000-3000',
    handler: async (p) => loadNapi().drift_taint(p.functionId as string),
  });
  register(catalog, {
    name: 'drift_dna_profile',
    description: 'Styling DNA profile for a module',
    category: 'detail',
    estimatedTokens: '~800-2000',
    handler: async (p) => loadNapi().drift_analyze(p.path as string),
  });
  register(catalog, {
    name: 'drift_wrappers',
    description: 'Framework wrapper detection',
    category: 'detail',
    estimatedTokens: '~500-1500',
    handler: async (p) => loadNapi().drift_boundaries(p.path as string),
  });

  // Analysis tools
  register(catalog, {
    name: 'drift_coupling',
    description: 'Module coupling analysis (Martin metrics)',
    category: 'analysis',
    estimatedTokens: '~1000-2500',
    handler: async (p) => loadNapi().drift_analyze(p.path as string),
  });
  register(catalog, {
    name: 'drift_test_topology',
    description: 'Test coverage and quality analysis',
    category: 'analysis',
    estimatedTokens: '~1000-2500',
    handler: async (p) => loadNapi().drift_test_topology(p.path as string),
  });
  register(catalog, {
    name: 'drift_error_handling',
    description: 'Error handling gap analysis',
    category: 'analysis',
    estimatedTokens: '~800-2000',
    handler: async (p) => loadNapi().drift_analyze(p.path as string),
  });
  register(catalog, {
    name: 'drift_quality_gate',
    description: 'Quality gate checks',
    category: 'analysis',
    estimatedTokens: '~1500-4000',
    handler: async (p) => loadNapi().drift_gates(p.path as string ?? '.'),
  });
  register(catalog, {
    name: 'drift_constants',
    description: 'Constants and secrets analysis',
    category: 'analysis',
    estimatedTokens: '~800-2000',
    handler: async (p) => loadNapi().drift_analyze(p.path as string),
  });
  register(catalog, {
    name: 'drift_constraints',
    description: 'Constraint verification',
    category: 'analysis',
    estimatedTokens: '~800-2000',
    handler: async (p) => loadNapi().drift_check(p.path as string),
  });
  register(catalog, {
    name: 'drift_audit',
    description: 'Full pattern audit with health scoring',
    category: 'analysis',
    estimatedTokens: '~1000-3000',
    handler: async (p) => loadNapi().drift_audit(p.path as string ?? '.'),
  });
  register(catalog, {
    name: 'drift_decisions',
    description: 'Decision mining from git history',
    category: 'analysis',
    estimatedTokens: '~800-2000',
    handler: async (p) => loadNapi().drift_decisions(p.path as string ?? '.'),
  });
  register(catalog, {
    name: 'drift_simulate',
    description: 'Speculative execution / Monte Carlo simulation',
    category: 'analysis',
    estimatedTokens: '~2000-5000',
    handler: async (p) => loadNapi().drift_simulate(p.task),
  });

  // Generation tools
  register(catalog, {
    name: 'drift_explain',
    description: 'Comprehensive code explanation',
    category: 'generation',
    estimatedTokens: '~2000-5000',
    handler: async (p) => loadNapi().drift_context(p.query as string ?? '', 'deep'),
  });
  register(catalog, {
    name: 'drift_validate_change',
    description: 'Validate code against patterns',
    category: 'generation',
    estimatedTokens: '~1000-3000',
    handler: async (p) => loadNapi().drift_check(p.path as string),
  });
  register(catalog, {
    name: 'drift_suggest_changes',
    description: 'Suggest pattern-aligned changes',
    category: 'generation',
    estimatedTokens: '~1000-3000',
    handler: async (p) => loadNapi().drift_violations(p.path as string),
  });

  // Setup tools
  register(catalog, {
    name: 'drift_generate_spec',
    description: 'Generate specification for a module',
    category: 'generation',
    estimatedTokens: '~1000-3000',
    handler: async (p) => loadNapi().drift_generate_spec(p.module as string),
  });

  return catalog;
}

function register(catalog: Map<string, InternalTool>, tool: InternalTool): void {
  catalog.set(tool.name, tool);
}

function listCapabilities(catalog: Map<string, InternalTool>): {
  tools: Array<{ name: string; description: string; category: string; estimatedTokens: string }>;
  totalCount: number;
} {
  const tools = Array.from(catalog.values()).map((t) => ({
    name: t.name,
    description: t.description,
    category: t.category,
    estimatedTokens: t.estimatedTokens,
  }));
  return { tools, totalCount: tools.length };
}

/**
 * Execute drift_tool — dynamic dispatch to internal tool.
 */
export async function handleDriftTool(
  params: DriftToolParams,
  catalog: Map<string, InternalTool>,
): Promise<unknown> {
  const tool = catalog.get(params.tool);
  if (!tool) {
    const available = Array.from(catalog.keys()).join(', ');
    throw new Error(
      `Unknown tool: "${params.tool}". Available tools: ${available}`,
    );
  }
  return tool.handler(params.params ?? {});
}
