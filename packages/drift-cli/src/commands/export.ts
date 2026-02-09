/**
 * drift export — export violations in any supported format.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';
import * as fs from 'node:fs';

export function registerExportCommand(program: Command): void {
  program
    .command('export [path]')
    .description('Export violations in the specified format')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'json')
    .option('-o, --output <file>', 'Write output to file instead of stdout')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (path: string | undefined, opts: { format: OutputFormat; output?: string; quiet?: boolean }) => {
      const napi = loadNapi();
      try {
        const violations = napi.drift_violations(path ?? process.cwd());
        const formatted = formatOutput(violations, opts.format);

        if (opts.output) {
          fs.writeFileSync(opts.output, formatted, 'utf-8');
          if (!opts.quiet) {
            process.stdout.write(`Exported to ${opts.output}\n`);
          }
        } else if (!opts.quiet) {
          process.stdout.write(formatted);
        }
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
