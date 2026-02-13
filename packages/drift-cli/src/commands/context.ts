/**
 * drift context — intent-weighted deep dive context generation.
 */

import type { Command } from 'commander';
import { loadNapi } from '../napi.js';
import { formatOutput, type OutputFormat } from '../output/index.js';
import { parseNapiJson } from '../output/parse-napi-json.js';

const VALID_INTENTS = [
  'fix_bug', 'add_feature', 'understand_code', 'security_audit', 'generate_spec',
] as const;

const VALID_DEPTHS = ['overview', 'standard', 'deep'] as const;

/**
 * Keyword → intent mapping.
 *
 * The Rust ContextIntent enum currently supports 5 intents. This map covers
 * common natural-language synonyms so users can type phrases like
 * "review my code", "refactor this", "debug the issue", etc. and get routed
 * to the closest supported intent. When the Rust side adds new intents,
 * add them to VALID_INTENTS above and update this map.
 */
const INTENT_KEYWORDS: Record<string, typeof VALID_INTENTS[number]> = {
  // fix_bug
  'fix': 'fix_bug',
  'bug': 'fix_bug',
  'debug': 'fix_bug',
  'repair': 'fix_bug',
  'patch': 'fix_bug',
  'error': 'fix_bug',
  'issue': 'fix_bug',
  'broken': 'fix_bug',
  'crash': 'fix_bug',
  'trace': 'fix_bug',
  'diagnose': 'fix_bug',
  // add_feature
  'add': 'add_feature',
  'feature': 'add_feature',
  'new': 'add_feature',
  'create': 'add_feature',
  'implement': 'add_feature',
  'build': 'add_feature',
  // understand_code — also covers review, refactor, explain, performance
  'understand': 'understand_code',
  'read': 'understand_code',
  'explore': 'understand_code',
  'learn': 'understand_code',
  'how': 'understand_code',
  'what': 'understand_code',
  'review': 'understand_code',
  'refactor': 'understand_code',
  'explain': 'understand_code',
  'performance': 'understand_code',
  'dependency': 'understand_code',
  'convention': 'understand_code',
  'boundary': 'understand_code',
  'coupling': 'understand_code',
  'documentation': 'understand_code',
  'risk': 'understand_code',
  // security_audit
  'security': 'security_audit',
  'audit': 'security_audit',
  'vulnerability': 'security_audit',
  'vuln': 'security_audit',
  'cve': 'security_audit',
  'owasp': 'security_audit',
  // generate_spec
  'spec': 'generate_spec',
  'specification': 'generate_spec',
  'document': 'generate_spec',
  'docs': 'generate_spec',
};

/**
 * Extended intent names that the Rust NAPI binding accepts as aliases.
 * These map to one of the 5 core intents on the Rust side.
 */
const RUST_INTENT_ALIASES: Record<string, typeof VALID_INTENTS[number]> = {
  'understand': 'understand_code',
  'review_code': 'understand_code',
  'review': 'understand_code',
  'refactor': 'understand_code',
  'explain_pattern': 'understand_code',
  'documentation': 'understand_code',
  'debug': 'fix_bug',
  'trace_dependency': 'fix_bug',
  'performance_audit': 'security_audit',
  'assess_risk': 'security_audit',
  'security': 'security_audit',
  'spec': 'generate_spec',
};

function resolveIntent(raw: string): string | null {
  // Exact match against core intents
  if (VALID_INTENTS.includes(raw as typeof VALID_INTENTS[number])) {
    return raw;
  }

  // Exact match against Rust-side aliases — pass through directly
  // so the Rust binding can map them to the correct ContextIntent.
  if (raw in RUST_INTENT_ALIASES) {
    return raw;
  }

  // Keyword matching — find best match from input words
  const words = raw.toLowerCase().replace(/[^a-z0-9_\s]/g, '').split(/\s+/);
  const matches = new Map<typeof VALID_INTENTS[number], number>();

  for (const word of words) {
    const mapped = INTENT_KEYWORDS[word];
    if (mapped) {
      matches.set(mapped, (matches.get(mapped) ?? 0) + 1);
    }
  }

  if (matches.size === 0) return null;

  // Return intent with most keyword hits
  return [...matches.entries()].sort((a, b) => b[1] - a[1])[0][0];
}

export function registerContextCommand(program: Command): void {
  program
    .command('context <intent>')
    .description('Generate intent-weighted context for a task')
    .option('-d, --depth <depth>', `Context depth: ${VALID_DEPTHS.join(', ')}`, 'standard')
    .option('--data <json>', 'Additional data as JSON string', '{}')
    .option('-f, --format <format>', 'Output format: table, json, sarif', 'json')
    .option('-q, --quiet', 'Suppress all output except errors')
    .action(async (intent: string, opts: { depth: string; data: string; format: OutputFormat; quiet?: boolean }) => {
      const napi = loadNapi();
      try {
        const resolved = resolveIntent(intent);
        if (!resolved) {
          const allIntents = [...VALID_INTENTS, ...Object.keys(RUST_INTENT_ALIASES)];
          process.stderr.write(
            `Could not resolve intent '${intent}'.\n` +
            `Valid intents: ${allIntents.join(', ')}\n` +
            `Tip: Use keywords like "fix", "add", "understand", "review", "refactor", "security", or "spec".\n`,
          );
          process.exitCode = 2;
          return;
        }

        if (resolved !== intent) {
          process.stderr.write(`Resolved intent: '${intent}' → '${resolved}'\n`);
        }

        if (!VALID_DEPTHS.includes(opts.depth as typeof VALID_DEPTHS[number])) {
          process.stderr.write(`Invalid depth '${opts.depth}'. Valid: ${VALID_DEPTHS.join(', ')}\n`);
          process.exitCode = 2;
          return;
        }
        const raw = await napi.driftContext(resolved, opts.depth, opts.data);
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
