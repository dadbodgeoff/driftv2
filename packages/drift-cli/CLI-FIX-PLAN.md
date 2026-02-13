# Drift CLI Bug Fix Plan

**Date:** 2026-02-13
**Scope:** 9 issues across `packages/drift-cli/src/` — 6 bugs, 3 minor fixes
**Baseline:** 47 tests passing (4 test files), `tsc` clean, NAPI release build clean

---

## Issue 1: Double-encoded JSON from string-returning NAPI methods

### Root Cause

`driftContext()`, `driftSimulate()`, and `driftGenerateSpec()` return `Promise<string>` — a JSON-serialized string from Rust. Three CLI commands pass this raw string directly to `formatOutput()`:

- `context.ts` line 99
- `simulate.ts` line 32
- `explain.ts` line 47

`formatOutput()` dispatches to:
- `formatJson()` → calls `JSON.stringify(data)` on an already-serialized string → double-encoded: `"\"{ \\\"sections\\\": ... }\""`
- `formatTable()` → calls `String(data) + '\n'` → dumps raw JSON string with no structure
- `formatSarif()` → calls `extractViolations(data)` → returns `[]` because a string has no `.violations` or `.results` property → empty SARIF

### Upstream Dependencies

- `driftContext` Rust binding in `crates/drift/drift-napi/src/bindings/advanced.rs` — returns `serde_json::to_string()`. No change needed.
- `driftSimulate` Rust binding in `crates/drift/drift-napi/src/bindings/advanced.rs` — same pattern. No change needed.
- `DriftNapi` interface in `packages/drift-napi-contracts/src/interface.ts` — declares `Promise<string>`. No change needed.
- Stub in `packages/drift-napi-contracts/src/stub.ts` — returns `JSON.stringify(...)`. No change needed.

### Downstream Dependencies

- `formatOutput()` in `packages/drift-cli/src/output/index.ts` — receives the parsed object instead of a string. No change needed (already handles objects correctly).
- Any consumer that pipes `drift context --format json` output into `jq` or another tool — currently broken, will be fixed.

### Fix

Create `packages/drift-cli/src/output/parse-napi-json.ts`:

```ts
/**
 * Parse a JSON string returned by NAPI bindings that serialize to String.
 * Returns the parsed object, or wraps the raw string in { raw: string }
 * so formatOutput always receives a structured value.
 */
export function parseNapiJson(raw: string): unknown {
  try {
    return JSON.parse(raw);
  } catch {
    return { raw };
  }
}
```

Modify 3 files:
- `src/commands/context.ts`: `const result = parseNapiJson(await napi.driftContext(...))` before `formatOutput(result, ...)`
- `src/commands/simulate.ts`: same pattern
- `src/commands/explain.ts`: same pattern

### Tests

```
T-FIX-01a: context command with stub NAPI → formatOutput receives parsed object, JSON output is valid single-encoded JSON
T-FIX-01b: simulate command with stub NAPI → same verification
T-FIX-01c: explain command with stub NAPI → same verification
T-FIX-01d: parseNapiJson with valid JSON string → returns parsed object
T-FIX-01e: parseNapiJson with non-JSON string → returns { raw: string }
T-FIX-01f: parseNapiJson with empty string → returns { raw: '' }
```

### Verification

- `drift context fix_bug --format json` output must be parseable by `JSON.parse()` in a single pass (not double-encoded).
- `drift context fix_bug --format table` output must show structured key-value pairs, not a raw JSON string blob.

---

## Issue 2: Dead `--require-native` flag

### Root Cause

In `src/index.ts` line 72, `program.opts()` is called before `program.parseAsync(process.argv)`. Commander hasn't parsed argv yet, so the returned options object contains only defaults. `opts.requireNative` is always `undefined`. The `--require-native` flag never fires.

### Upstream Dependencies

- `createProgram()` in `src/index.ts` — registers the `--require-native` option on the program. No change needed.
- `isNapiStub()` from `@drift/napi-contracts` — pure function, no change needed.

### Downstream Dependencies

- CI pipelines that pass `--require-native` to ensure native binary is present — currently silently ignored, will start working.
- Any user relying on the flag to fail-fast when native binary is missing — currently broken.

### Fix

Remove the dead `program.opts()` block (lines 72–80 of `src/index.ts`). Replace with a Commander `hook('preAction')` on the program, which fires after parsing but before any command action:

```ts
program.hook('preAction', () => {
  const globalOpts = program.opts();
  if (globalOpts.requireNative && isNapiStub()) {
    process.stderr.write(
      'Error: --require-native specified but native binary is unavailable. ' +
      'Install platform-specific binary or run `napi build`.\n',
    );
    process.exitCode = 2;
    throw new Error('Native binary required but unavailable');
  }
});
```

The thrown error is caught by the existing try/catch around `program.parseAsync()`.

### Tests

```
T-FIX-02a: program.hook('preAction') is registered (structural check)
T-FIX-02b: with stub NAPI + --require-native → preAction hook throws, exitCode = 2
T-FIX-02c: with stub NAPI + no --require-native → preAction hook does not throw
```

### Verification

- `drift --require-native status` with stub NAPI must print error and exit 2.
- `drift status` with stub NAPI must work normally (no error).

---

## Issue 3: `cloud push` wrong argument order for `driftCloudReadRows`

### Root Cause

In `src/commands/cloud.ts` line 147, the reader calls:
```ts
napiAny.driftCloudReadRows(projectRoot, table, db, afterCursor ?? 0)
```

But the interface signature is:
```ts
driftCloudReadRows(table: string, db: string, afterCursor?: number, limit?: number): unknown[]
```

`projectRoot` is passed as `table`, `table` as `db`, `db` as `afterCursor`. Every row read queries the wrong table/db combination.

Similarly, line 153:
```ts
napiAny.driftCloudMaxCursor(projectRoot, db)
```

But the interface is:
```ts
driftCloudMaxCursor(db: string): number
```

`projectRoot` is passed where `db` is expected, and `db` is an extra arg that gets ignored.

### Upstream Dependencies

- `driftCloudReadRows` Rust binding — takes `(table, db, after_cursor, limit)`. The runtime already knows the project root from `driftInitialize()`. No change needed.
- `driftCloudMaxCursor` Rust binding — takes `(db)`. Same. No change needed.
- `DriftNapi` interface — correctly typed. No change needed.
- Stub — correctly typed. No change needed.

### Downstream Dependencies

- `SyncClient.push()` from `@drift/core/cloud` — receives the reader object. It calls `reader.readRows(table, db, cursor)` and `reader.getMaxCursor(db)`. The fix aligns the reader implementation with what `SyncClient` expects.
- Cloud sync data integrity — currently pushing wrong/empty data. Will push correct data after fix.

### Fix

In `src/commands/cloud.ts`, remove `projectRoot` from both calls:

```ts
const reader = {
  readRows: async (table: string, db: string, afterCursor?: number) => {
    try {
      if (typeof napiAny.driftCloudReadRows !== 'function') return [];
      return napiAny.driftCloudReadRows(table, db, afterCursor ?? 0) as Record<string, unknown>[];
    } catch {
      return [];
    }
  },
  getMaxCursor: async (db: string) => {
    try {
      if (typeof napiAny.driftCloudMaxCursor !== 'function') return 0;
      return napiAny.driftCloudMaxCursor(db) as number;
    } catch {
      return 0;
    }
  },
};
```

### Tests

```
T-FIX-03a: cloud push reader.readRows calls driftCloudReadRows with (table, db, cursor) — not (projectRoot, table, db, cursor)
T-FIX-03b: cloud push reader.getMaxCursor calls driftCloudMaxCursor with (db) — not (projectRoot, db)
```

### Verification

- Spy on `driftCloudReadRows` and `driftCloudMaxCursor` during a mock push. Verify first arg is a table name (e.g. `"file_metadata"`), not a path.

---

## Issue 4: `bridge events --tier` filter is a no-op

### Root Cause

In `src/commands/bridge.ts` line 456, the filter expression is:
```ts
m.description.toLowerCase().includes(opts.tier!.toLowerCase()) || true
```

The `|| true` makes every element pass the filter. The filtered list is always identical to the unfiltered list.

### Upstream Dependencies

- `driftBridgeEventMappings()` — returns `BridgeEventMappingsResult` with `mappings: BridgeEventMapping[]`. Each mapping has `description`, `importance`, `event_type`, `memory_type`. No change needed.

### Downstream Dependencies

- CLI output — currently shows all mappings regardless of `--tier`. After fix, shows only matching mappings.

### Fix

Remove `|| true`. Add empty-result feedback:

```ts
if (opts.tier) {
  const tierLower = opts.tier.toLowerCase();
  mappings = mappings.filter(
    (m: BridgeEventMapping) => m.description.toLowerCase().includes(tierLower),
  );
  if (mappings.length === 0) {
    process.stderr.write(`No event mappings match tier '${opts.tier}'.\n`);
  }
}
```

Import `BridgeEventMapping` from `@drift/napi-contracts` for proper typing instead of the inline type annotation.

### Tests

```
T-FIX-04a: bridge events --tier with matching description → returns filtered subset
T-FIX-04b: bridge events --tier with no matches → returns empty, prints hint
T-FIX-04c: bridge events without --tier → returns all mappings (no filter applied)
```

---

## Issue 5: `bridge memories` triggers grounding + ignores filters

### Root Cause

In `src/commands/bridge.ts` line 160, the `memories` subcommand calls `driftBridgeGroundAll()` — a write/mutating operation that re-validates all memories against drift.db evidence. A listing command should be read-only.

Additionally, `--type`, `--verdict`, and `--limit` options are collected into the output object but never used to filter data.

### Upstream Dependencies

- `driftBridgeGroundAll()` — mutating operation. Should NOT be called from a read-only listing command.
- `driftBridgeStatus()` — read-only, returns availability and tier info.
- `driftBridgeHealth()` — read-only, returns subsystem health.

### Downstream Dependencies

- Users running `drift bridge memories` expect a read-only listing, not a side-effecting grounding pass.
- The `--type`, `--verdict`, `--limit` flags are documented in `--help` but do nothing — misleading UX.

### Fix

Replace the `driftBridgeGroundAll()` call with read-only `driftBridgeStatus()` + `driftBridgeHealth()`. Since there is no dedicated "list memories with details" NAPI binding, the command should honestly report what it can without side effects, and direct users to `drift bridge ground` for the grounding operation and `drift bridge history <id>` for per-memory details:

```ts
.action(async (opts) => {
  const napi = loadNapi();
  try {
    const status = napi.driftBridgeStatus();
    if (!status.available) {
      process.stderr.write('Bridge not initialized, run drift setup\n');
      process.exitCode = 1;
      return;
    }
    const health = napi.driftBridgeHealth();
    const result = {
      bridge_status: status.available ? 'active' : 'inactive',
      license_tier: status.license_tier,
      grounding_enabled: status.grounding_enabled,
      health_status: health.status,
      ready: health.ready,
      subsystems: health.subsystem_checks.length,
      filter: {
        type: opts.type ?? 'all',
        verdict: opts.verdict ?? 'all',
        limit: parseInt(opts.limit, 10),
      },
      hint: 'Run `drift bridge ground` to trigger grounding, or `drift bridge history <memoryId>` for per-memory details.',
    };
    process.stdout.write(formatOutput(result, opts.format));
  } catch (err) {
    process.stderr.write(`Error: ${err instanceof Error ? err.message : err}\n`);
    process.exitCode = 2;
  }
});
```

### Tests

```
T-FIX-05a: bridge memories does NOT call driftBridgeGroundAll (spy verification)
T-FIX-05b: bridge memories calls driftBridgeStatus + driftBridgeHealth (spy verification)
T-FIX-05c: bridge memories output includes hint about ground and history commands
```

---

## Issue 6: `scan` multi-path merge is lossy

### Root Cause

In `src/commands/scan.ts` lines 56–65, when scanning multiple paths, only `filesTotal` is accumulated. All other fields (`filesAdded`, `filesModified`, `filesRemoved`, `filesUnchanged`, `errorsCount`, `durationMs`, `languages`) come from only the last scan result.

### Upstream Dependencies

- `driftScan()` — returns `ScanSummary` per path. No change needed.
- `ScanSummary` type — has fields: `filesTotal`, `filesAdded`, `filesModified`, `filesRemoved`, `filesUnchanged`, `errorsCount`, `durationMs`, `status: string`, `languages: Record<string, number>`. All numeric fields must be summed. `status` should be `'complete'` unless any scan fails. `languages` maps must be merged (sum counts per language).

### Downstream Dependencies

- CLI output for `drift scan src/ lib/` — currently shows misleading partial stats. Will show correct merged stats.
- Any CI script parsing scan output — will see correct totals.

### Fix

Replace the scan loop with proper accumulation:

```ts
import type { ScanSummary } from '@drift/napi-contracts';

const merged: ScanSummary = {
  filesTotal: 0,
  filesAdded: 0,
  filesModified: 0,
  filesRemoved: 0,
  filesUnchanged: 0,
  errorsCount: 0,
  durationMs: 0,
  status: 'complete',
  languages: {},
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
```

This also simplifies the output path — no more separate single-path vs multi-path branches.

### Tests

```
T-FIX-06a: single-path scan → merged result equals the single scan result
T-FIX-06b: multi-path scan (2 paths) → all numeric fields are summed correctly
T-FIX-06c: multi-path scan → languages maps are merged (overlapping keys summed)
T-FIX-06d: multi-path scan → if any scan status != 'complete', merged status reflects it
```

---

## Issue 7: JSDoc comment count mismatch

### Root Cause

`src/commands/index.ts` line 3 says "29 CLI commands" but 32 are registered (including cortex, bridge, validate-pack, cloud).

### Fix

Change the JSDoc to match reality. Count: scan, analyze, check, status, report, patterns, violations, security, contracts, coupling, dna, taint, errors, test-quality, impact, fix, dismiss, suppress, explain, approve, simulate, context, audit, export, gc, setup, doctor, cortex, bridge, validate-pack, cloud = 31 top-level commands.

Update: `"29 CLI commands"` → `"31 CLI commands"`.

### Tests

```
T-FIX-07a: existing test T8-CLI-01 already asserts toHaveLength(31) — no new test needed, just verify it still passes.
```

---

## Issue 8: `export` command dead fallback branch + unused `path` arg

### Root Cause

The `else` branch (line 33) that calls `driftViolations(path)` can never execute because `REPORT_FORMATS` includes `'json'` (the default), and the `if` check is `REPORT_FORMATS.includes(opts.format)`. Any format the user passes either matches the list (goes through `driftReport`) or is an unknown format that Commander still accepts as a string. The `path` argument is also unused in the primary `driftReport()` branch.

### Fix

Add format validation before the branch (like `report.ts` does), and remove the dead `else`:

```ts
if (!REPORT_FORMATS.includes(opts.format as typeof REPORT_FORMATS[number])) {
  process.stderr.write(`Invalid format '${opts.format}'. Valid: ${REPORT_FORMATS.join(', ')}\n`);
  process.exitCode = 2;
  return;
}
const formatted = napi.driftReport(opts.format);
```

Remove the `[path]` argument from the command definition since `driftReport()` doesn't take a path — it reads from the initialized drift.db.

### Tests

```
T-FIX-08a: export with valid format calls driftReport (spy verification)
T-FIX-08b: export with invalid format prints error and exits 2
```

---

## Issue 9: Cortex commands use exit code 1 instead of 2 for errors

### Root Cause

All cortex subcommands use `process.exitCode = 1` in their catch blocks. Every other drift command uses `process.exitCode = 2` for errors. The CLI convention is: 0 = success, 1 = violations/findings found, 2 = runtime error.

### Upstream Dependencies

- None — this is a CLI-only convention.

### Downstream Dependencies

- CI scripts that check exit codes to distinguish "violations found" (1) from "command failed" (2). Cortex errors currently look like "violations found" instead of "command failed".

### Fix

Global find-and-replace in `src/commands/cortex.ts`: change all `process.exitCode = 1` in catch blocks to `process.exitCode = 2`.

Count: there are approximately 30+ catch blocks in cortex.ts that need this change.

### Tests

```
T-FIX-09a: structural check — grep cortex.ts for "exitCode = 1" in catch blocks, expect 0 occurrences
```

---

## Execution Order

The fixes are independent — no fix depends on another. However, for clean git history:

1. **Issue 1** (parseNapiJson) — new file + 3 command changes. Highest user impact.
2. **Issue 3** (cloud push args) — 1 file, data integrity fix.
3. **Issue 6** (scan merge) — 1 file, correctness fix.
4. **Issue 2** (require-native) — 1 file, flag fix.
5. **Issue 5** (bridge memories) — 1 file, side-effect removal.
6. **Issue 4** (bridge events filter) — 1 file, filter fix.
7. **Issue 8** (export dead branch) — 1 file, cleanup.
8. **Issue 9** (cortex exit codes) — 1 file, convention alignment.
9. **Issue 7** (JSDoc count) — 1 line, trivial.

---

## Test File

All new tests go in `packages/drift-cli/tests/commands/cli_bugfixes.test.ts`.

Test infrastructure: use the existing `createStubNapi()` + `setNapi()` + `vi.fn()` spy pattern established in `napi_alignment.test.ts`.

Total new tests: 22

---

## Files Modified (Summary)

| File | Change |
|------|--------|
| `src/output/parse-napi-json.ts` | NEW — parseNapiJson utility |
| `src/commands/context.ts` | Parse JSON before formatOutput |
| `src/commands/simulate.ts` | Parse JSON before formatOutput |
| `src/commands/explain.ts` | Parse JSON before formatOutput |
| `src/index.ts` | Replace dead opts check with preAction hook |
| `src/commands/cloud.ts` | Fix driftCloudReadRows/MaxCursor arg order |
| `src/commands/bridge.ts` | Fix events filter, fix memories side-effect |
| `src/commands/scan.ts` | Fix multi-path merge accumulation |
| `src/commands/export.ts` | Remove dead branch, add format validation |
| `src/commands/index.ts` | Fix JSDoc count |
| `src/commands/cortex.ts` | Fix exit codes 1 → 2 in catch blocks |
| `tests/commands/cli_bugfixes.test.ts` | NEW — 22 regression tests |

## Validation Checklist

- [ ] `npm run build` (tsc) passes with zero errors
- [ ] `npm run test` passes — all 47 existing tests + 22 new tests
- [ ] `getDiagnostics` on all modified files shows zero issues
- [ ] Manual smoke: `drift context fix_bug -f json` produces single-encoded JSON
- [ ] Manual smoke: `drift scan src/ lib/` shows correct merged totals
- [ ] Manual smoke: `drift --require-native status` (with stub) exits 2
