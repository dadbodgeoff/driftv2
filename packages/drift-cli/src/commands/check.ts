/**
 * drift check — run quality gates and report violations.
 *
 * Exit codes: 0 = clean, 1 = violations found, 2 = error.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';

export function registerCheckCommand(program: Command): void {
  program
    .command('check [path]')
    .description('Run quality gates and check for violations')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('-p, --policy <policy>', 'Policy to apply: strict, standard, lenient')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (path: string | undefined, opts: { format: OutputFormat; policy?: string; quiet?: boolean }) => {
      const napi = loadNapi();
      const checkPath = path ?? process.cwd();

      try {
        const result = napi.drift_check(checkPath, opts.policy);
        if (!opts.quiet) {
          process.stdout.write(formatOutput(result, opts.format));
        }
        process.exitCode = result.passed ? 0 : 1;
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
