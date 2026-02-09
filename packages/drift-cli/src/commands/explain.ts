/**
 * drift explain — human-readable explanation with remediation steps.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';

export function registerExplainCommand(program: Command): void {
  program
    .command('explain <violationId>')
    .description('Get a human-readable explanation of a violation with remediation steps')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (violationId: string, opts: { format: OutputFormat; quiet?: boolean }) => {
      const napi = loadNapi();
      try {
        // Use context generation with the violation ID as intent
        const result = napi.drift_context(`explain_violation:${violationId}`, 'deep');
        if (!opts.quiet) {
          process.stdout.write(formatOutput(result, opts.format));
        }
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
