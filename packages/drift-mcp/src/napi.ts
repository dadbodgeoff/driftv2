/**
 * NAPI bridge interface — thin wrapper around drift-napi bindings.
 *
 * In production, this loads the native module. For testing and when
 * the native module is not available, it provides graceful fallbacks.
 */

import type {
  StatusOverview,
  ContextOutput,
  ScanResult,
  DriftContextParams,
  DriftScanParams,
} from './types.js';

/** NAPI binding interface — all functions that drift-napi exposes. */
export interface DriftNapi {
  drift_init(config?: Record<string, unknown>): void;
  drift_shutdown(): void;
  drift_status(): StatusOverview;
  drift_scan(path: string, options?: Record<string, unknown>): ScanResult;
  drift_context(intent: string, depth: string): ContextOutput;
  drift_analyze(path: string): unknown;
  drift_check(path: string, policy?: string): unknown;
  drift_violations(path: string): unknown[];
  drift_patterns(path: string): unknown;
  drift_reachability(functionId: string): unknown;
  drift_taint(functionId: string): unknown;
  drift_impact(functionId: string): unknown;
  drift_test_topology(path: string): unknown;
  drift_audit(path: string): unknown;
  drift_simulate(task: unknown): unknown;
  drift_decisions(path: string): unknown[];
  drift_boundaries(path: string): unknown;
  drift_call_graph(path: string): unknown;
  drift_confidence(patternId: string): unknown;
  drift_gates(path: string): unknown[];
  drift_generate_spec(module: string): unknown;
}

let _napi: DriftNapi | null = null;

/**
 * Load the NAPI bindings. Tries to require the native module,
 * falls back to a stub for environments without native bindings.
 */
export function loadNapi(): DriftNapi {
  if (_napi) return _napi;

  try {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const native = require('drift-napi') as DriftNapi;
    _napi = native;
    return native;
  } catch {
    // Native module not available — return stub for testing
    _napi = createStubNapi();
    return _napi;
  }
}

/** Inject a custom NAPI implementation (for testing). */
export function setNapi(napi: DriftNapi): void {
  _napi = napi;
}

/** Reset NAPI to force re-loading. */
export function resetNapi(): void {
  _napi = null;
}

function createStubNapi(): DriftNapi {
  return {
    drift_init() {},
    drift_shutdown() {},
    drift_status(): StatusOverview {
      return {
        version: '2.0.0',
        projectRoot: process.cwd(),
        fileCount: 0,
        patternCount: 0,
        violationCount: 0,
        healthScore: 100,
        lastScanTime: null,
        gateStatus: 'unknown',
      };
    },
    drift_scan(_path: string): ScanResult {
      return { filesScanned: 0, patternsDetected: 0, violationsFound: 0, durationMs: 0 };
    },
    drift_context(intent: string, depth: string): ContextOutput {
      return { intent, depth, sections: [], tokenCount: 0, truncated: false };
    },
    drift_analyze() { return {}; },
    drift_check() { return { passed: true, violations: [] }; },
    drift_violations() { return []; },
    drift_patterns() { return { patterns: [] }; },
    drift_reachability() { return { reachable: [] }; },
    drift_taint() { return { flows: [] }; },
    drift_impact() { return { affected: [] }; },
    drift_test_topology() { return { coverage: 0, tests: [] }; },
    drift_audit() { return { healthScore: 100, issues: [] }; },
    drift_simulate() { return { result: 'ok' }; },
    drift_decisions() { return []; },
    drift_boundaries() { return { boundaries: [] }; },
    drift_call_graph() { return { nodes: [], edges: [] }; },
    drift_confidence() { return { confidence: 0.5 }; },
    drift_gates() { return []; },
    drift_generate_spec() { return { sections: [] }; },
  };
}
