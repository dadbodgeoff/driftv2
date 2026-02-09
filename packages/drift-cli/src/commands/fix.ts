/**
 * drift fix — apply quick fixes for violations.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';

export function registerFixCommand(program: Command): void {
  program
    .command('fix [path]')
    .description('Show available quick fixes for violations')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('--dry-run', 'Show what would be fixed without applying changes')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (path: string | undefined, opts: { format: OutputFormat; dryRun?: boolean; quiet?: boolean }) => {
      const napi = loadNapi();
      try {
        const violations = napi.drift_violations(path ?? process.cwd());
        // Filter to violations with quick fixes
        const fixable = (violations as Array<{ quick_fix?: unknown }>).filter(
          (v) => v.quick_fix != null,
        );

        if (fixable.length === 0) {
          if (!opts.quiet) {
            process.stdout.write('No auto-fixable violations found.\n');
          }
          process.exitCode = 0;
          return;
        }

        if (opts.dryRun) {
          if (!opts.quiet) {
            process.stdout.write(`Found ${fixable.length} fixable violation(s):\n`);
            process.stdout.write(formatOutput(fixable, opts.format));
          }
        } else {
          if (!opts.quiet) {
            process.stdout.write(`${fixable.length} violation(s) have available fixes.\n`);
            process.stdout.write(formatOutput(fixable, opts.format));
          }
        }
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
