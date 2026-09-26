import type { DiagnosticsSnapshot } from "./diagnostics.types";

export const diagnosticsVisualFixture: DiagnosticsSnapshot = {
  installationId: 1,
  registryState: "ready",
  registryDetail: "Local diagnostic candidates were checked against canonical artifact identities.",
  reports: [
    {
      diagnosticId: 41,
      sourceKind: "mccc",
      reportName: "mc_lastexception.html",
      parseStatus: "parsed",
      observedAt: "unix-ms:1780000000000",
      contentHash: "fixture",
      observations: [
        {
          kind: "module",
          value: "mc_cmd_center",
          context: "module: mc_cmd_center"
        },
        {
          kind: "file",
          value: "mc_cmd_center.ts4script",
          context: "mc_cmd_center.ts4script"
        }
      ],
      candidates: [
        {
          localFileId: 9,
          relativePath: "MCCC/mc_cmd_center.ts4script",
          confidence: "exact",
          confidenceScore: 99,
          relationship: "implicated_by_diagnostic_evidence",
          evidence: [
            {
              kind: "filename",
              reference: "mc_cmd_center.ts4script",
              explanation: "Diagnostic report names this installed artifact file directly."
            },
            {
              kind: "module",
              reference: "mc_cmd_center",
              explanation: "Diagnostic module exactly matches a module exposed by this installed TS4Script archive."
            }
          ],
          canonical: {
            modId: "fixture-mod",
            releaseId: "fixture-release",
            confidence: "1.0",
            deterministic: true
          }
        }
      ],
      note: "Candidates are correlated with the report evidence; this is not a causality claim.",
      telemetryPreview: {
        sourceKind: "mccc",
        observationKinds: ["file", "module"],
        candidateCount: 1,
        redactionsApplied: 2
      }
    },
    {
      diagnosticId: 42,
      sourceKind: "last_ui_exception",
      reportName: "lastUIException.txt",
      parseStatus: "parsed",
      observedAt: "unix-ms:1780000001000",
      contentHash: "fixture-2",
      observations: [],
      candidates: [],
      note: "The report parsed, but no supported mod/module/resource reference was found.",
      telemetryPreview: {
        sourceKind: "last_ui_exception",
        observationKinds: [],
        candidateCount: 0,
        redactionsApplied: 0
      }
    }
  ]
};
