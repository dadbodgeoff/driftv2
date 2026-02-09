/**
 * Tool registration — registers all MCP tools on the server.
 *
 * Progressive disclosure architecture:
 * - 4 registered MCP tools: drift_status, drift_context, drift_scan, drift_tool
 * - ~49 internal tools accessible via drift_tool dynamic dispatch
 * - Reduces token overhead ~81% compared to registering all tools individually
 */

import type { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { handleDriftStatus, DRIFT_STATUS_SCHEMA } from './drift_status.js';
import { handleDriftContext, DRIFT_CONTEXT_SCHEMA } from './drift_context.js';
import { handleDriftScan, DRIFT_SCAN_SCHEMA } from './drift_scan.js';
import {
  handleDriftTool,
  buildToolCatalog,
  DRIFT_TOOL_SCHEMA,
} from './drift_tool.js';
import type { InternalTool } from '../types.js';

/**
 * Register all MCP tools on the server instance.
 * Returns the internal tool catalog for drift_tool dispatch.
 */
export function registerTools(server: McpServer): Map<string, InternalTool> {
  const catalog = buildToolCatalog();

  // Entry point 1: drift_status — overview, <1ms
  server.tool(
    'drift_status',
    'Get project overview — file count, pattern count, violations, health score, gate status. Reads materialized view for <1ms response.',
    DRIFT_STATUS_SCHEMA,
    async () => {
      const result = await handleDriftStatus();
      return { content: [{ type: 'text' as const, text: JSON.stringify(result, null, 2) }] };
    },
  );

  // Entry point 2: drift_context — intent-weighted deep dive
  server.tool(
    'drift_context',
    'Get intent-weighted context for your current task. Replaces 3-5 individual tool calls with a single curated response. Supports shallow/standard/deep depth levels.',
    DRIFT_CONTEXT_SCHEMA,
    async (params) => {
      const result = await handleDriftContext(params as Parameters<typeof handleDriftContext>[0]);
      return { content: [{ type: 'text' as const, text: JSON.stringify(result, null, 2) }] };
    },
  );

  // Entry point 3: drift_scan — trigger analysis
  server.tool(
    'drift_scan',
    'Trigger analysis on the project. Scans files, detects patterns, identifies violations. Supports incremental mode for faster re-scans.',
    DRIFT_SCAN_SCHEMA,
    async (params) => {
      const result = await handleDriftScan(params as Parameters<typeof handleDriftScan>[0]);
      return { content: [{ type: 'text' as const, text: JSON.stringify(result, null, 2) }] };
    },
  );

  // Entry point 4: drift_tool — dynamic dispatch for ~49 internal tools
  server.tool(
    'drift_tool',
    'Access any of ~49 internal analysis tools by name. Use drift_status to discover available tools. Supports: reachability, taint, impact, coupling, test_topology, error_handling, patterns, security, audit, simulate, decisions, and more.',
    DRIFT_TOOL_SCHEMA,
    async (params) => {
      const result = await handleDriftTool(
        params as Parameters<typeof handleDriftTool>[0],
        catalog,
      );
      return { content: [{ type: 'text' as const, text: JSON.stringify(result, null, 2) }] };
    },
  );

  return catalog;
}

export { handleDriftStatus } from './drift_status.js';
export { handleDriftContext } from './drift_context.js';
export { handleDriftScan } from './drift_scan.js';
export { handleDriftTool, buildToolCatalog } from './drift_tool.js';
