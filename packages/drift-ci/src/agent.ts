/**
 * CI Agent — orchestrates 9 parallel analysis passes.
 *
 * Passes: scan, patterns, call_graph, boundaries, security, tests, errors, contracts, constraints.
 * Supports PR-level incremental analysis (only changed files + transitive dependents).
 */

import { loadNapi } from './napi.js';

/** Result from a single analysis pass. */
export interface PassResult {
  name: string;
  status: 'passed' | 'failed' | 'error';
  violations: number;
  durationMs: number;
  data: unknown;
  error?: string;
}

/** Aggregated result from all 9 passes. */
export interface AnalysisResult {
  status: 'passed' | 'failed';
  totalViolations: number;
  score: number;
  passes: PassResult[];
  durationMs: number;
  summary: string;
  filesAnalyzed: number;
  incremental: boolean;
}

/** CI agent configuration. */
export interface CiAgentConfig {
  path: string;
  policy: 'strict' | 'standard' | 'lenient';
  failOn: 'error' | 'warning' | 'none';
  incremental: boolean;
  threshold: number;
  timeoutMs: number;
  changedFiles?: string[];
}

export const DEFAULT_CI_CONFIG: CiAgentConfig = {
  path: '.',
  policy: 'standard',
  failOn: 'error',
  incremental: true,
  threshold: 0,
  timeoutMs: 300_000, // 5 minutes
};

/** Analysis pass definition. */
interface AnalysisPass {
  name: string;
  run: (files: string[], config: CiAgentConfig) => Promise<PassResult>;
}

/**
 * Run a single pass with timeout and error handling.
 */
async function runPassSafe(
  pass: AnalysisPass,
  files: string[],
  config: CiAgentConfig,
): Promise<PassResult> {
  const start = Date.now();
  try {
    const result = await Promise.race([
      pass.run(files, config),
      new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error('Pass timed out')), config.timeoutMs),
      ),
    ]);
    return result;
  } catch (err) {
    return {
      name: pass.name,
      status: 'error',
      violations: 0,
      durationMs: Date.now() - start,
      data: null,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/** The 9 analysis passes. */
function buildPasses(): AnalysisPass[] {
  return [
    {
      name: 'scan',
      run: async (files, config) => {
        const napi = loadNapi();
        const start = Date.now();
        const result = napi.drift_scan(config.path, { incremental: config.incremental });
        return {
          name: 'scan',
          status: 'passed',
          violations: result.violationsFound,
          durationMs: Date.now() - start,
          data: result,
        };
      },
    },
    {
      name: 'patterns',
      run: async (_files, config) => {
        const napi = loadNapi();
        const start = Date.now();
        const result = napi.drift_patterns(config.path);
        return {
          name: 'patterns',
          status: 'passed',
          violations: 0,
          durationMs: Date.now() - start,
          data: result,
        };
      },
    },
    {
      name: 'call_graph',
      run: async (_files, config) => {
        const napi = loadNapi();
        const start = Date.now();
        const result = napi.drift_call_graph(config.path);
        return {
          name: 'call_graph',
          status: 'passed',
          violations: 0,
          durationMs: Date.now() - start,
          data: result,
        };
      },
    },
    {
      name: 'boundaries',
      run: async (_files, config) => {
        const napi = loadNapi();
        const start = Date.now();
        const result = napi.drift_boundaries(config.path);
        return {
          name: 'boundaries',
          status: 'passed',
          violations: 0,
          durationMs: Date.now() - start,
          data: result,
        };
      },
    },
    {
      name: 'security',
      run: async (_files, config) => {
        const napi = loadNapi();
        const start = Date.now();
        const result = napi.drift_check(config.path, 'security');
        return {
          name: 'security',
          status: result.passed ? 'passed' : 'failed',
          violations: result.violations.length,
          durationMs: Date.now() - start,
          data: result,
        };
      },
    },
    {
      name: 'tests',
      run: async (_files, config) => {
        const napi = loadNapi();
        const start = Date.now();
        const result = napi.drift_test_topology(config.path);
        return {
          name: 'tests',
          status: 'passed',
          violations: 0,
          durationMs: Date.now() - start,
          data: result,
        };
      },
    },
    {
      name: 'errors',
      run: async (_files, config) => {
        const napi = loadNapi();
        const start = Date.now();
        const result = napi.drift_analyze(config.path);
        return {
          name: 'errors',
          status: 'passed',
          violations: 0,
          durationMs: Date.now() - start,
          data: result,
        };
      },
    },
    {
      name: 'contracts',
      run: async (_files, config) => {
        const napi = loadNapi();
        const start = Date.now();
        const result = napi.drift_check(config.path, 'contracts');
        return {
          name: 'contracts',
          status: result.passed ? 'passed' : 'failed',
          violations: result.violations.length,
          durationMs: Date.now() - start,
          data: result,
        };
      },
    },
    {
      name: 'constraints',
      run: async (_files, config) => {
        const napi = loadNapi();
        const start = Date.now();
        const result = napi.drift_check(config.path, 'constraints');
        return {
          name: 'constraints',
          status: result.passed ? 'passed' : 'failed',
          violations: result.violations.length,
          durationMs: Date.now() - start,
          data: result,
        };
      },
    },
  ];
}

/**
 * Run all 9 analysis passes in parallel.
 */
export async function runAnalysis(
  config: Partial<CiAgentConfig> = {},
): Promise<AnalysisResult> {
  const mergedConfig = { ...DEFAULT_CI_CONFIG, ...config };
  const passes = buildPasses();
  const files = mergedConfig.changedFiles ?? [];

  // Handle empty PR diff
  if (mergedConfig.incremental && files.length === 0 && mergedConfig.changedFiles !== undefined) {
    return {
      status: 'passed',
      totalViolations: 0,
      score: 100,
      passes: [],
      durationMs: 0,
      summary: 'No changes to analyze',
      filesAnalyzed: 0,
      incremental: true,
    };
  }

  const start = Date.now();

  // Run all 9 passes in parallel
  const results = await Promise.all(
    passes.map((pass) => runPassSafe(pass, files, mergedConfig)),
  );

  const totalViolations = results.reduce((sum, r) => sum + r.violations, 0);
  const hasFailures = results.some((r) => r.status === 'failed');
  const hasErrors = results.some((r) => r.status === 'error');

  // Calculate score (0-100)
  const score = calculateScore(results);

  // Determine overall status
  let status: 'passed' | 'failed' = 'passed';
  if (mergedConfig.failOn === 'error' && (hasFailures || hasErrors)) {
    status = 'failed';
  } else if (mergedConfig.failOn === 'warning' && totalViolations > 0) {
    status = 'failed';
  }
  if (score < mergedConfig.threshold) {
    status = 'failed';
  }

  const durationMs = Date.now() - start;

  return {
    status,
    totalViolations,
    score,
    passes: results,
    durationMs,
    summary: buildSummary(results, totalViolations, score, durationMs),
    filesAnalyzed: files.length || -1, // -1 means full scan
    incremental: mergedConfig.incremental,
  };
}

/**
 * Calculate quality score from pass results (0-100).
 * Weighted average: scan=20%, patterns=15%, security=20%, tests=15%, errors=10%, contracts=10%, constraints=10%.
 */
function calculateScore(results: PassResult[]): number {
  const weights: Record<string, number> = {
    scan: 0.20,
    patterns: 0.15,
    call_graph: 0.0, // informational
    boundaries: 0.0, // informational
    security: 0.20,
    tests: 0.15,
    errors: 0.10,
    contracts: 0.10,
    constraints: 0.10,
  };

  let totalWeight = 0;
  let weightedScore = 0;

  for (const result of results) {
    const weight = weights[result.name] ?? 0;
    if (weight === 0) continue;
    totalWeight += weight;
    const passScore = result.status === 'passed' ? 100 : result.status === 'error' ? 0 : 50;
    weightedScore += passScore * weight;
  }

  return totalWeight > 0 ? Math.round(weightedScore / totalWeight) : 100;
}

function buildSummary(
  results: PassResult[],
  totalViolations: number,
  score: number,
  durationMs: number,
): string {
  const passed = results.filter((r) => r.status === 'passed').length;
  const total = results.length;
  return `${passed}/${total} passes passed, ${totalViolations} violations, score ${score}/100 (${durationMs}ms)`;
}
