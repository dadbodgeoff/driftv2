/**
 * MCP Server — main server class.
 *
 * Sets up the MCP server with progressive disclosure:
 * - 4 registered tools (drift_status, drift_context, drift_scan, drift_tool)
 * - ~49 internal tools via drift_tool dynamic dispatch
 * - stdio transport (primary) + Streamable HTTP transport (secondary)
 * - MCP protocol compliant via @modelcontextprotocol/sdk
 */

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import type { Transport } from '@modelcontextprotocol/sdk/shared/transport.js';
import { registerTools } from './tools/index.js';
import { loadNapi } from './napi.js';
import type { McpConfig, InternalTool } from './types.js';
import { DEFAULT_MCP_CONFIG } from './types.js';

export interface DriftMcpServer {
  /** The underlying MCP server instance. */
  server: McpServer;
  /** Internal tool catalog for drift_tool dispatch. */
  catalog: Map<string, InternalTool>;
  /** Connect to a transport and start serving. */
  connect(transport: Transport): Promise<void>;
  /** Graceful shutdown. */
  close(): Promise<void>;
}

/**
 * Create and configure the Drift MCP server.
 *
 * @param config - Server configuration (transport, token limits, etc.)
 * @returns Configured server ready to connect to a transport
 */
export function createDriftMcpServer(
  config: Partial<McpConfig> = {},
): DriftMcpServer {
  const mergedConfig = { ...DEFAULT_MCP_CONFIG, ...config };

  // Initialize NAPI bindings
  const napi = loadNapi();
  try {
    napi.drift_init(
      mergedConfig.projectRoot
        ? { projectRoot: mergedConfig.projectRoot }
        : undefined,
    );
  } catch {
    // Non-fatal — NAPI may already be initialized or not available
  }

  // Create MCP server
  const server = new McpServer({
    name: 'drift-analysis',
    version: '2.0.0',
  });

  // Register all tools (progressive disclosure)
  const catalog = registerTools(server);

  return {
    server,
    catalog,
    async connect(transport: Transport): Promise<void> {
      await server.connect(transport);
    },
    async close(): Promise<void> {
      try {
        await server.close();
      } finally {
        try {
          napi.drift_shutdown();
        } catch {
          // Non-fatal
        }
      }
    },
  };
}
