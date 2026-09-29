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
    conflicts: 18,
    unknown: 25
  },
  conflictAggregation: {
    exactDuplicateGroupCount: 18,
    potentialConflictGroupCount: 2,
    attentionGroupCount: 18,
    rawOverlapPairCount: 47,
    suppressedDuplicateOverlapPairCount: 6,
    potentialConflictGroups: [
      {
        classification: "potentialConflictGroup",
        confidence: "low",
        countsTowardAttention: false,
        fileIds: [101, 102, 103],
        relativePaths: [
          "CreatorA/CAS-eyes.package",
          "CreatorB/CAS-eyes-overlay.package",
          "CreatorC/CAS-eyes-default.package"
        ],
        overlapPairCount: 3,
        sharedResourceCount: 12,
        sampleResourceKeys: [
          { resourceType: 3451, group: 0, instance: 101 }
        ],
        sampleOverlapPairs: [
          {
            classification: "potentialConflict",
            leftFileId: 101,
            leftRelativePath: "CreatorA/CAS-eyes.package",
            rightFileId: 102,
            rightRelativePath: "CreatorB/CAS-eyes-overlay.package",
            sharedResourceCount: 5,
            sampleResourceKeys: [
              { resourceType: 3451, group: 0, instance: 101 }
            ]
          }
        ]
      },
      {
        classification: "potentialConflictGroup",
        confidence: "low",
        countsTowardAttention: false,
        fileIds: [201, 202],
        relativePaths: [
          "BuildBuy/window.package",
          "BuildBuy/window-recolor.package"
        ],
        overlapPairCount: 1,
        sharedResourceCount: 4,
        sampleResourceKeys: [
          { resourceType: 319, group: 0, instance: 202 }
        ],
        sampleOverlapPairs: []
      }
    ]
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
