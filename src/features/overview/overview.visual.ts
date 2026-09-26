import type { OverviewSnapshot } from "./overview.types";

export const overviewVisualFixture: OverviewSnapshot = {
  hasInstallation: true,
  gameVersion: "1.128.90",
  platform: "windows",
  indexedCount: 324,
  healthScore: 87,
  healthScoreExplanation:
    "Overall health is the percentage of current canonical releases with verified compatible patch evidence. Update-available releases still count as compatible when their installed release is compatible; unresolved files remain in the denominator.",
  healthCounts: {
    healthy: 267,
    updates: 21,
    conflicts: 7,
    unknown: 25
  },
  attentionCount: 28,
  attention: [
    {
      name: "MC Command Center",
      creator: "Registry health",
      detail: "Update available to 2026.5.0; installed compatibility is compatible.",
      badge: "Update",
      tone: "update"
    },
    {
      name: "Relationship & Pregnancy Overhaul",
      creator: "Registry health",
      detail: "No current-patch compatibility evidence is available.",
      badge: "Unknown",
      tone: "muted"
    },
    {
      name: "Duplicate CAS package",
      creator: "Local scan",
      detail: "2 exact copies share the same SHA-256 fingerprint.",
      badge: "Duplicate",
      tone: "warning"
    }
  ],
  installation: {
    scriptMods: 43,
    packageFiles: 177,
    unidentified: 25,
    exactDuplicates: 18
  },
  registryState: "ready",
  registryDetail: "Local scan, artifact identity and registry health are current.",
  scan: {
    scanSessionId: 42,
    status: "completed",
    startedAt: "2026-09-26T20:00:00Z",
    completedAt: "2026-09-26T20:02:00Z",
    filesSeen: 324,
    filesHashed: 17,
    filesSkipped: 307,
    observations: 0,
    stale: false,
    partial: false
  }
};
