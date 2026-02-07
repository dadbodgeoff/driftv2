/**
 * drift_knowledge_timeline — Knowledge evolution visualization over time.
 */

import type { CortexClient } from "../../bridge/client.js";
import type { DriftSnapshot, McpToolDefinition } from "../../bridge/types.js";

type Granularity = "hourly" | "daily" | "weekly";

function getIntervalMs(granularity: Granularity): number {
  switch (granularity) {
    case "hourly":
      return 60 * 60 * 1000;
    case "daily":
      return 24 * 60 * 60 * 1000;
    case "weekly":
      return 7 * 24 * 60 * 60 * 1000;
  }
}

function computeTrend(snapshots: DriftSnapshot[]): {
  ksi_trend: string;
  confidence_trend: string;
  freshness_trend: string;
} {
  if (snapshots.length < 2) {
    return { ksi_trend: "insufficient_data", confidence_trend: "insufficient_data", freshness_trend: "insufficient_data" };
  }

  const first = snapshots[0];
  const last = snapshots[snapshots.length - 1];

  const classify = (delta: number): string =>
    delta > 0.05 ? "improving" : delta < -0.05 ? "declining" : "stable";

  return {
    ksi_trend: classify(last.global.overall_ksi - first.global.overall_ksi),
    confidence_trend: classify(last.global.avg_confidence - first.global.avg_confidence),
    freshness_trend: classify(
      last.global.overall_evidence_freshness - first.global.overall_evidence_freshness,
    ),
  };
}

export function driftKnowledgeTimeline(client: CortexClient): McpToolDefinition {
  return {
    name: "drift_knowledge_timeline",
    description:
      "Visualize knowledge evolution over time. Returns a time-series of drift " +
      "snapshots at the specified granularity with trend analysis.",
    inputSchema: {
      type: "object",
      properties: {
        from: {
          type: "string",
          description: "ISO 8601 timestamp — start of timeline.",
        },
        to: {
          type: "string",
          description: "ISO 8601 timestamp — end of timeline.",
        },
        granularity: {
          type: "string",
          enum: ["hourly", "daily", "weekly"],
          description: "Time granularity for snapshots (default: daily).",
        },
      },
      required: ["from", "to"],
    },
    handler: async (args) => {
      const fromTime = new Date(args.from as string).getTime();
      const toTime = new Date(args.to as string).getTime();
      const granularity = ((args.granularity as string) ?? "daily") as Granularity;
      const intervalMs = getIntervalMs(granularity);

      // Compute window hours for each snapshot based on granularity
      const windowHours = Math.ceil(intervalMs / (60 * 60 * 1000));

      const snapshots: DriftSnapshot[] = [];
      for (let t = fromTime; t <= toTime; t += intervalMs) {
        const snapshot = await client.getDriftMetrics(windowHours);
        snapshots.push(snapshot);
      }

      return {
        snapshots,
        trend: computeTrend(snapshots),
      };
    },
  };
}
