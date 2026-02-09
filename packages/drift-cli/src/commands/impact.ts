/**
 * drift impact — analyze change blast radius.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';

export function registerImpactCommand(program: Command): void {
  program
    .command('impact <functionId>')
    .description('Analyze change blast radius for a function')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (functionId: string, opts: { format: OutputFormat; quiet?: boolean }) => {
      const napi = loadNapi();
      try {
        const result = napi.drift_impact(functionId);
        if (!opts.quiet) {
          process.stdout.write(formatOutput(result, opts.format));
        }
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
