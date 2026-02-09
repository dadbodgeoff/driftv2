/**
 * drift violations — list all violations.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';

export function registerViolationsCommand(program: Command): void {
  program
    .command('violations [path]')
    .description('List all violations found in the project')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (path: string | undefined, opts: { format: OutputFormat; quiet?: boolean }) => {
      const napi = loadNapi();
      try {
        const result = napi.drift_violations(path ?? process.cwd());
        if (!opts.quiet) {
          process.stdout.write(formatOutput(result, opts.format));
        }
        process.exitCode = (result as unknown[]).length > 0 ? 1 : 0;
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
