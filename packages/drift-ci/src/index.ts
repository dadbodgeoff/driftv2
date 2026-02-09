#!/usr/bin/env node
/**
 * Drift CI Agent — entry point.
 *
 * 9 parallel analysis passes, PR-level incremental analysis,
 * SARIF upload to GitHub Code Scanning, PR comment generation.
 *
 * Usage:
 *   drift-ci analyze --path . --policy standard --fail-on error
 */

import { runAnalysis, type CiAgentConfig, DEFAULT_CI_CONFIG } from './agent.js';
import { generatePrComment } from './pr_comment.js';
import { uploadSarif, writeSarifFile } from './sarif_upload.js';
import { loadNapi } from './napi.js';
import * as fs from 'node:fs';

// Re-export public API
export { runAnalysis, type AnalysisResult, type PassResult, type CiAgentConfig } from './agent.js';
export { generatePrComment, type PrComment } from './pr_comment.js';
export { uploadSarif, writeSarifFile, type SarifUploadConfig, type SarifUploadResult } from './sarif_upload.js';
export { setNapi } from './napi.js';
export type { DriftNapi } from './napi.js';

/**
 * Parse CLI arguments.
 */
function parseArgs(args: string[]): {
  config: Partial<CiAgentConfig>;
  outputSarif?: string;
  outputJson?: string;
} {
  const config: Partial<CiAgentConfig> = {};
  let outputSarif: string | undefined;
  let outputJson: string | undefined;

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    switch (arg) {
      case '--path':
        config.path = args[++i];
        break;
      case '--policy':
        config.policy = args[++i] as CiAgentConfig['policy'];
        break;
      case '--fail-on':
        config.failOn = args[++i] as CiAgentConfig['failOn'];
        break;
      case '--threshold':
        config.threshold = parseInt(args[++i], 10);
        break;
      case '--incremental':
        config.incremental = true;
        break;
      case '--timeout':
        config.timeoutMs = parseInt(args[++i], 10) * 1000;
        break;
      case '--output-sarif':
        outputSarif = args[++i];
        break;
      case '--output-json':
        outputJson = args[++i];
        break;
    }
  }

  return { config, outputSarif, outputJson };
}

/**
 * Main entry point.
 */
async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const command = args[0];

  if (command !== 'analyze') {
    process.stderr.write('Usage: drift-ci analyze [options]\n');
    process.stderr.write('Options:\n');
    process.stderr.write('  --path <path>          Path to analyze\n');
    process.stderr.write('  --policy <policy>      Policy: strict, standard, lenient\n');
    process.stderr.write('  --fail-on <level>      Fail on: error, warning, none\n');
    process.stderr.write('  --threshold <score>    Minimum quality score (0-100)\n');
    process.stderr.write('  --incremental          Only analyze changed files\n');
    process.stderr.write('  --timeout <seconds>    Analysis timeout\n');
    process.stderr.write('  --output-sarif <path>  Write SARIF output to file\n');
    process.stderr.write('  --output-json <path>   Write JSON output to file\n');
    process.exitCode = 2;
    return;
  }

  // Initialize NAPI
  const napi = loadNapi();
  try {
    napi.drift_init();
  } catch {
    // Non-fatal
  }

  const { config, outputSarif, outputJson } = parseArgs(args.slice(1));

  // Run analysis
  const result = await runAnalysis(config);

  // Generate PR comment
  const comment = generatePrComment(result);

  // Write outputs
  if (outputJson) {
    const dir = outputJson.substring(0, outputJson.lastIndexOf('/'));
    if (dir && !fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }
    fs.writeFileSync(outputJson, JSON.stringify(result, null, 2), 'utf-8');
  }

  if (outputSarif) {
    const dir = outputSarif.substring(0, outputSarif.lastIndexOf('/'));
    if (dir && !fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }
    const violations = napi.drift_violations(config.path ?? '.');
    writeSarifFile(violations, outputSarif);
  }

  // Print summary
  process.stdout.write(comment.markdown);
  process.stdout.write('\n');

  // Exit code
  process.exitCode = result.status === 'passed' ? 0 : 1;
}

// Run if executed directly
const isMainModule =
  typeof process !== 'undefined' &&
  process.argv[1] &&
  (process.argv[1].endsWith('drift-ci') ||
    process.argv[1].endsWith('index.js') ||
    process.argv[1].endsWith('index.ts'));

if (isMainModule) {
  main().catch((err) => {
    process.stderr.write(`Fatal: ${err}\n`);
    process.exit(2);
  });
}
