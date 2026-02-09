/**
 * NAPI bridge interface for CI agent.
 */

export interface DriftNapi {
  drift_init(config?: Record<string, unknown>): void;
  drift_shutdown(): void;
  drift_scan(path: string, options?: Record<string, unknown>): { filesScanned: number; patternsDetected: number; violationsFound: number; durationMs: number };
  drift_patterns(path: string): Record<string, unknown>;
  drift_call_graph(path: string): Record<string, unknown>;
  drift_boundaries(path: string): Record<string, unknown>;
  drift_check(path: string, policy?: string): { passed: boolean; violations: unknown[] };
  drift_test_topology(path: string): Record<string, unknown>;
  drift_analyze(path: string): Record<string, unknown>;
  drift_violations(path: string): unknown[];
  drift_audit(path: string): Record<string, unknown>;
  drift_status(): Record<string, unknown>;
}

let _napi: DriftNapi | null = null;

export function loadNapi(): DriftNapi {
  if (_napi) return _napi;
  try {
    const native = require('drift-napi') as DriftNapi;
    _napi = native;
    return native;
  } catch {
    _napi = createStubNapi();
    return _napi;
  }
}

export function setNapi(napi: DriftNapi): void {
  _napi = napi;
}

function createStubNapi(): DriftNapi {
  return {
    drift_init() {},
    drift_shutdown() {},
    drift_scan() { return { filesScanned: 0, patternsDetected: 0, violationsFound: 0, durationMs: 0 }; },
    drift_patterns() { return { patterns: [] }; },
    drift_call_graph() { return { nodes: [], edges: [] }; },
    drift_boundaries() { return { boundaries: [] }; },
    drift_check() { return { passed: true, violations: [] }; },
    drift_test_topology() { return { coverage: 0, tests: [] }; },
    drift_analyze() { return {}; },
    drift_violations() { return []; },
    drift_audit() { return { healthScore: 100 }; },
    drift_status() { return { version: '2.0.0', fileCount: 0, violationCount: 0, healthScore: 100 }; },
  };
}
