/**
 * drift scan — scan project for patterns and violations.
 *
 * Supports --include and --exclude glob flags for folder selection,
 * plus persistent configuration via drift.toml [scan] section.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';
import * as fs from 'node:fs';

export function registerScanCommand(program: Command): void {
  program
    .command('scan [paths...]')
    .description('Scan project for patterns and violations. Pass one or more folder paths to scan specific directories.')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('-i, --incremental', 'Only scan changed files since last scan')
    .option('--include <globs...>', 'Only scan files matching these glob patterns (e.g., "src/**")')
    .option('--exclude <globs...>', 'Exclude files matching these glob patterns (e.g., "drift/**")')
    .option('--follow-symlinks', 'Follow symbolic links during scan')
    .option('--max-file-size <bytes>', 'Maximum file size in bytes (default: 1MB)', parseInt)
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (paths: string[], opts: {
      format: OutputFormat;
      incremental?: boolean;
      include?: string[];
      exclude?: string[];
      followSymlinks?: boolean;
      maxFileSize?: number;
      quiet?: boolean;
    }) => {
      const napi = loadNapi();
      const scanPaths = paths.length > 0 ? paths : [process.cwd()];

      // Validate that all scan paths exist before scanning.
      // Without this, the scanner sees all previously-known files as "removed"
      // and returns a misleading filesRemoved count with exit 0.
      const invalid = scanPaths.filter((p) => !fs.existsSync(p));
      if (invalid.length > 0) {
        process.stderr.write(
          `Error: path${invalid.length > 1 ? 's' : ''} not found: ${invalid.join(', ')}\n`,
        );
        process.exitCode = 2;
        return;
      }

      // Build scan options from CLI flags
      const options: Record<string, unknown> = {};
      if (opts.incremental === true) {
        options.forceFull = false;
      }
      if (opts.include && opts.include.length > 0) {
        options.include = opts.include;
      }
      if (opts.exclude && opts.exclude.length > 0) {
        options.extraIgnore = opts.exclude;
      }
      if (opts.followSymlinks === true) {
        options.followSymlinks = true;
      }
      if (opts.maxFileSize !== undefined) {
        options.maxFileSize = opts.maxFileSize;
      }

      const scanOptions = Object.keys(options).length > 0 ? options : undefined;

      try {
        // Accumulate results across all scan paths
        const merged = {
          filesTotal: 0,
          filesAdded: 0,
          filesModified: 0,
          filesRemoved: 0,
          filesUnchanged: 0,
          errorsCount: 0,
          durationMs: 0,
          status: 'complete' as string,
          languages: {} as Record<string, number>,
        };

        for (const scanPath of scanPaths) {
          const result = await napi.driftScan(scanPath, scanOptions as Parameters<typeof napi.driftScan>[1]);
          merged.filesTotal += result.filesTotal;
          merged.filesAdded += result.filesAdded;
          merged.filesModified += result.filesModified;
          merged.filesRemoved += result.filesRemoved;
          merged.filesUnchanged += result.filesUnchanged;
          merged.errorsCount += result.errorsCount;
          merged.durationMs += result.durationMs;
          if (result.status !== 'complete') merged.status = result.status;
          for (const [lang, count] of Object.entries(result.languages)) {
            merged.languages[lang] = (merged.languages[lang] ?? 0) + count;
          }
        }

        if (!opts.quiet) {
          process.stdout.write(formatOutput(merged, opts.format));
        }
        process.exitCode = 0;
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
