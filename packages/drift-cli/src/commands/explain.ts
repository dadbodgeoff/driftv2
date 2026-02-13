/**
 * drift explain — human-readable explanation with remediation steps.
 *
 * Looks up the violation by ID, gathers full project context from drift.db,
 * and generates an intent-weighted explanation with remediation guidance.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';
import { parseNapiJson } from '../output/parse-napi-json.js';

export function registerExplainCommand(program: Command): void {
  program
    .command('explain <violationId>')
    .description('Get a human-readable explanation of a violation with remediation steps')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'table')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (violationId: string, opts: { format: OutputFormat; quiet?: boolean }) => {
      const napi = loadNapi();
      try {
        // Step 1: Look up the violation to get its details
        const violations = napi.driftViolations('.');
        const violation = violations.find(
          (v) => v.id === violationId,
        );

        // Step 2: Build explicit data with the violation context.
        // Pass an empty object so drift_context auto-gathers from drift.db,
        // then the violation details are added as an explicit section override.
        const data: Record<string, string> = {};
        if (violation) {
          const details = [
            `Violation: ${violation.id}`,
            `Rule: ${violation.ruleId || 'unknown'}`,
            `File: ${violation.file || 'unknown'}`,
            `Line: ${violation.line || 'unknown'}`,
            `Severity: ${violation.severity || 'unknown'}`,
            `Message: ${violation.message || 'No message'}`,
          ].join('\n');
          data.violation_context = details;
        } else {
          data.violation_context = `Violation '${violationId}' not found in current scan results. Run 'drift analyze --scan' first.`;
        }

        const raw = await napi.driftContext('understand_code', 'deep', JSON.stringify(data));
        const result = parseNapiJson(raw);
        if (!opts.quiet) {
          process.stdout.write(formatOutput(result, opts.format));
        }
      } catch (err) {
        process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
        process.exitCode = 2;
      }
    });
}
