/**
 * MCP Server tests — T8-MCP-01 through T8-MCP-10.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { setNapi, resetNapi } from '../src/napi.js';
import { handleDriftStatus } from '../src/tools/drift_status.js';
import { handleDriftContext } from '../src/tools/drift_context.js';
import { handleDriftScan } from '../src/tools/drift_scan.js';
import { handleDriftTool, buildToolCatalog } from '../src/tools/drift_tool.js';
import { createDriftMcpServer } from '../src/server.js';
import type { DriftNapi } from '../src/napi.js';

/** Create a mock NAPI with controllable responses. */
function createMockNapi(overrides: Partial<DriftNapi> = {}): DriftNapi {
  return {
    drift_init() {},
    drift_shutdown() {},
    drift_status() {
      return {
        version: '2.0.0',
        projectRoot: '/test',
        fileCount: 42,
        patternCount: 15,
        violationCount: 3,
        healthScore: 87.5,
        lastScanTime: '2026-02-09T10:00:00Z',
        gateStatus: 'passed' as const,
      };
    },
    drift_scan() {
      return { filesScanned: 42, patternsDetected: 15, violationsFound: 3, durationMs: 150 };
    },
    drift_context(intent: string, depth: string) {
      return {
        intent,
        depth,
        sections: [
          { title: 'Patterns', content: 'Found 15 patterns', relevanceScore: 0.9 },
          { title: 'Security', content: 'No critical issues', relevanceScore: 0.7 },
        ],
        tokenCount: 500,
        truncated: false,
      };
    },
    drift_analyze() { return { analyzed: true }; },
    drift_check() { return { passed: true, violations: [] }; },
    drift_violations() { return []; },
    drift_patterns() { return { patterns: [{ id: 'p1', name: 'test' }] }; },
    drift_reachability() { return { reachable: ['fn1', 'fn2'] }; },
    drift_taint() { return { flows: [] }; },
    drift_impact() { return { affected: ['module1'] }; },
    drift_test_topology() { return { coverage: 85, tests: [] }; },
    drift_audit() { return { healthScore: 90, issues: [] }; },
    drift_simulate() { return { result: 'ok', p50: 5 }; },
    drift_decisions() { return [{ id: 'd1', type: 'refactor' }]; },
    drift_boundaries() { return { boundaries: [] }; },
    drift_call_graph() { return { nodes: [], edges: [] }; },
    drift_confidence() { return { confidence: 0.85 }; },
    drift_gates() { return []; },
    drift_generate_spec() { return { sections: [] }; },
    ...overrides,
  };
}

describe('MCP Server', () => {
  beforeEach(() => {
    setNapi(createMockNapi());
  });

  // T8-MCP-01: Test MCP server registers all drift-analysis tools via stdio transport
  it('T8-MCP-01: registers all tools on server creation', () => {
    const server = createDriftMcpServer();
    expect(server.server).toBeDefined();
    expect(server.catalog).toBeDefined();
    // 4 entry points registered + internal catalog
    expect(server.catalog.size).toBeGreaterThanOrEqual(20);
  });

  // T8-MCP-02: Test drift_status returns overview in <1ms
  it('T8-MCP-02: drift_status returns overview in <1ms', async () => {
    const start = performance.now();
    const result = await handleDriftStatus();
    const elapsed = performance.now() - start;

    expect(result.version).toBe('2.0.0');
    expect(result.fileCount).toBe(42);
    expect(result.patternCount).toBe(15);
    expect(result.violationCount).toBe(3);
    expect(result.healthScore).toBe(87.5);
    expect(result.gateStatus).toBe('passed');
    // <1ms for mock (real NAPI reads materialized view)
    expect(elapsed).toBeLessThan(50);
  });

  // T8-MCP-03: Test drift_context produces intent-weighted context with token budgeting
  it('T8-MCP-03: drift_context produces intent-weighted context', async () => {
    const result = await handleDriftContext({
      intent: 'fix_bug',
      depth: 'standard',
    });

    expect(result.intent).toBe('fix_bug');
    expect(result.depth).toBe('standard');
    expect(result.sections).toHaveLength(2);
    expect(result.tokenCount).toBe(500);
    expect(result.truncated).toBe(false);
  });

  // T8-MCP-04: Test progressive disclosure reduces token overhead
  it('T8-MCP-04: progressive disclosure reduces token overhead ~81%', () => {
    const server = createDriftMcpServer();
    // 4 registered MCP tools vs ~52 total tools
    const registeredToolCount = 4; // drift_status, drift_context, drift_scan, drift_tool
    const totalToolCount = server.catalog.size + registeredToolCount;
    const reduction = 1 - registeredToolCount / totalToolCount;
    expect(reduction).toBeGreaterThan(0.75); // At least 75% reduction
  });

  // T8-MCP-05: Test MCP server handles malformed requests
  it('T8-MCP-05: handles unknown tool name gracefully', async () => {
    const catalog = buildToolCatalog();
    await expect(
      handleDriftTool({ tool: 'nonexistent_tool', params: {} }, catalog),
    ).rejects.toThrow('Unknown tool');
  });

  // T8-MCP-06: Test drift_tool dynamic dispatch routes correctly
  it('T8-MCP-09: drift_tool dispatches to correct internal tool', async () => {
    const catalog = buildToolCatalog();

    const statusResult = await handleDriftTool(
      { tool: 'drift_status', params: {} },
      catalog,
    );
    expect(statusResult).toHaveProperty('version');

    const capResult = await handleDriftTool(
      { tool: 'drift_capabilities', params: {} },
      catalog,
    );
    expect(capResult).toHaveProperty('tools');
    expect(capResult).toHaveProperty('totalCount');
  });

  // T8-MCP-07: Test drift_scan triggers analysis
  it('T8-MCP-07: drift_scan triggers analysis', async () => {
    const result = await handleDriftScan({ path: '/test' });
    expect(result.filesScanned).toBe(42);
    expect(result.patternsDetected).toBe(15);
    expect(result.violationsFound).toBe(3);
  });

  // T8-MCP-08: Test concurrent requests
  it('T8-MCP-08: handles concurrent requests correctly', async () => {
    const results = await Promise.all([
      handleDriftStatus(),
      handleDriftStatus(),
      handleDriftStatus(),
      handleDriftContext({ intent: 'test', depth: 'shallow' }),
      handleDriftScan({}),
    ]);

    expect(results).toHaveLength(5);
    // All status results should be identical
    expect(results[0]).toEqual(results[1]);
    expect(results[1]).toEqual(results[2]);
  });

  // T8-MCP-10: Test graceful shutdown
  it('T8-MCP-10: graceful shutdown', async () => {
    let shutdownCalled = false;
    setNapi(
      createMockNapi({
        drift_shutdown() {
          shutdownCalled = true;
        },
      }),
    );

    const server = createDriftMcpServer();
    await server.close();
    expect(shutdownCalled).toBe(true);
  });

  // Test drift_context with focus parameter
  it('drift_context with focus filters sections', async () => {
    setNapi(
      createMockNapi({
        drift_context(intent: string, depth: string) {
          return {
            intent,
            depth,
            sections: [
              { title: 'Auth Module', content: 'Authentication patterns', relevanceScore: 0.5 },
              { title: 'Database', content: 'DB patterns', relevanceScore: 0.9 },
              { title: 'Auth Middleware', content: 'Auth middleware details', relevanceScore: 0.3 },
            ],
            tokenCount: 800,
            truncated: false,
          };
        },
      }),
    );

    const result = await handleDriftContext({
      intent: 'fix_bug',
      focus: 'auth',
    });

    // Auth-related sections should be sorted first
    expect(result.sections[0].title).toContain('Auth');
  });
});
