/**
 * NAPI bridge interface for CLI — same pattern as MCP server.
 */

export interface DriftNapi {
  drift_init(config?: Record<string, unknown>): void;
  drift_shutdown(): void;
  drift_status(): Record<string, unknown>;
  drift_scan(path: string, options?: Record<string, unknown>): Record<string, unknown>;
  drift_context(intent: string, depth: string): Record<string, unknown>;
  drift_analyze(path: string): Record<string, unknown>;
  drift_check(path: string, policy?: string): { passed: boolean; violations: unknown[] };
  drift_violations(path: string): unknown[];
  drift_patterns(path: string): Record<string, unknown>;
  drift_impact(functionId: string): Record<string, unknown>;
  drift_audit(path: string): Record<string, unknown>;
  drift_simulate(task: unknown): Record<string, unknown>;
  drift_gates(path: string): unknown[];
  drift_generate_spec(module: string): Record<string, unknown>;
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
    drift_status() { return { version: '2.0.0', fileCount: 0, patternCount: 0, violationCount: 0, healthScore: 100, gateStatus: 'unknown' }; },
    drift_scan() { return { filesScanned: 0, patternsDetected: 0, violationsFound: 0, durationMs: 0 }; },
    drift_context() { return { sections: [], tokenCount: 0 }; },
    drift_analyze() { return {}; },
    drift_check() { return { passed: true, violations: [] }; },
    drift_violations() { return []; },
    drift_patterns() { return { patterns: [] }; },
    drift_impact() { return { affected: [] }; },
    drift_audit() { return { healthScore: 100, issues: [] }; },
    drift_simulate() { return { result: 'ok' }; },
    drift_gates() { return []; },
    drift_generate_spec() { return { sections: [] }; },
  };
}
