/**
 * drift simulate — run speculative execution / Monte Carlo simulation.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';

export function registerSimulateCommand(program: Command): void {
  program
    .command('simulate <task>')
    .description('Run speculative execution simulation for a task')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (task: string, opts: { format: OutputFormat; quiet?: boolean }) => {
      const napi = loadNapi();
      try {
        const result = napi.drift_simulate({ description: task });
        if (!opts.quiet) {
          process.stdout.write(formatOutput(result, opts.format));
        }
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
