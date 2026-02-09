/**
 * drift patterns — list detected patterns.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';

export function registerPatternsCommand(program: Command): void {
  program
    .command('patterns [path]')
    .description('List detected code patterns')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (path: string | undefined, opts: { format: OutputFormat; quiet?: boolean }) => {
      const napi = loadNapi();
      try {
        const result = napi.drift_patterns(path ?? process.cwd());
        if (!opts.quiet) {
          process.stdout.write(formatOutput(result, opts.format));
        }
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
