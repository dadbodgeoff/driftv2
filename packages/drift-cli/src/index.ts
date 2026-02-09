#!/usr/bin/env node
/**
 * Drift CLI — entry point.
 *
 * 13 commands: scan, check, status, patterns, violations, impact,
 * simulate, audit, setup, doctor, export, explain, fix.
 *
 * Exit codes: 0 = clean, 1 = violations found, 2 = error.
 */

import { Command } from 'commander';
import { registerAllCommands } from './commands/index.js';
import { loadNapi } from './napi.js';

// Re-export public API
export { registerAllCommands } from './commands/index.js';
export { formatOutput } from './output/index.js';
export type { OutputFormat } from './output/index.js';
export { setNapi } from './napi.js';
export type { DriftNapi } from './napi.js';

/**
 * Create and configure the CLI program.
 */
export function createProgram(): Command {
  const program = new Command();

  program
    .name('drift')
    .description('Drift — AI-native code analysis and quality enforcement')
    .version('2.0.0')
    .option('-q, --quiet', 'Suppress all output except errors')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table');

  registerAllCommands(program);

  return program;
}

/**
 * Main entry point — parses args and runs the appropriate command.
 */
async function main(): Promise<void> {
  // Initialize NAPI
  const napi = loadNapi();
  try {
    napi.drift_init();
  } catch {
    // Non-fatal — may not be initialized yet (drift setup handles this)
  }

  const program = createProgram();

  try {
    await program.parseAsync(process.argv);
  } catch (err) {
    process.stderr.write(
      `Error: ${err instanceof Error ? err.message : err}\n`,
    );
    process.exitCode = 2;
  }
}

// Run if executed directly
const isMainModule =
  typeof process !== 'undefined' &&
  process.argv[1] &&
  (process.argv[1].endsWith('drift') ||
    process.argv[1].endsWith('index.js') ||
    process.argv[1].endsWith('index.ts'));

if (isMainModule) {
  main().catch((err) => {
    process.stderr.write(`Fatal: ${err}\n`);
    process.exit(2);
  });
}
