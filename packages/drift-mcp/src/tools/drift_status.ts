/**
 * drift_status — project overview from materialized_status view.
 *
 * Performance target: <1ms (reads pre-computed materialized view).
 * Returns: version, file count, pattern count, violation count, health score, gate status.
 */

import { loadNapi } from '../napi.js';
import type { StatusOverview } from '../types.js';

/** JSON Schema for drift_status parameters. */
export const DRIFT_STATUS_SCHEMA = {
  type: 'object' as const,
  properties: {},
  additionalProperties: false,
};

/**
 * Execute drift_status — reads materialized_status view for <1ms response.
 */
export async function handleDriftStatus(): Promise<StatusOverview> {
  const napi = loadNapi();
  return napi.drift_status();
}
