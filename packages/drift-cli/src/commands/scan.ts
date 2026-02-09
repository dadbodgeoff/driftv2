/**
 * drift scan — scan project for patterns and violations.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';

export function registerScanCommand(program: Command): void {
  program
    .command('scan [path]')
    .description('Scan project for patterns and violations')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('-i, --incremental', 'Only scan changed files since last scan')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (path: string | undefined, opts: { format: OutputFormat; incremental?: boolean; quiet?: boolean }) => {
      const napi = loadNapi();
      const scanPath = path ?? process.cwd();
      const options = opts.incremental ? { incremental: true } : undefined;

      try {
        const result = napi.drift_scan(scanPath, options);
        if (!opts.quiet) {
          process.stdout.write(formatOutput(result, opts.format));
        }
        process.exitCode = 0;
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
