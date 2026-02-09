/**
 * drift_scan — trigger analysis on the project.
 *
 * Calls NAPI drift_scan() to run the full analysis pipeline.
 * Supports incremental mode for faster re-scans.
 */

import { loadNapi } from '../napi.js';
import type { DriftScanParams, ScanResult } from '../types.js';

/** JSON Schema for drift_scan parameters. */
export const DRIFT_SCAN_SCHEMA = {
  type: 'object' as const,
  properties: {
    path: {
      type: 'string',
      description: 'Path to scan (defaults to project root)',
    },
    incremental: {
      type: 'boolean',
      description: 'Only scan changed files since last scan',
      default: false,
    },
  },
  additionalProperties: false,
};

/**
 * Execute drift_scan — triggers analysis pipeline.
 */
export async function handleDriftScan(
  params: DriftScanParams,
): Promise<ScanResult> {
  const napi = loadNapi();
  const scanPath = params.path ?? process.cwd();
  const options = params.incremental ? { incremental: true } : undefined;
  return napi.drift_scan(scanPath, options);
}
