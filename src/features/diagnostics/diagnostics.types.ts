export type DiagnosticEvidence = {
  kind: string;
  reference: string;
  explanation: string;
};

export type CanonicalDiagnosticIdentity = {
  modId: string;
  releaseId: string;
  confidence: string;
  deterministic: boolean;
};

export type DiagnosticCandidate = {
  localFileId: number;
  relativePath: string;
  confidence: "exact" | "high" | "medium" | string;
  confidenceScore: number;
  relationship: "implicated_by_diagnostic_evidence" | string;
  evidence: DiagnosticEvidence[];
  canonical: CanonicalDiagnosticIdentity | null;
};

export type DiagnosticObservation = {
  kind: string;
  value: string;
  context: string;
};

export type DiagnosticReport = {
  diagnosticId: number | null;
  sourceKind: string;
  reportName: string;
  parseStatus: "parsed" | "unsupported" | "malformed" | "error" | string;
  observedAt: string;
  contentHash: string | null;
  observations: DiagnosticObservation[];
  candidates: DiagnosticCandidate[];
  note: string;
  telemetryPreview: {
    sourceKind: string;
    observationKinds: string[];
    candidateCount: number;
    redactionsApplied: number;
  };
};

export type DiagnosticsSnapshot = {
  installationId: number | null;
  registryState: "ready" | "offline" | "partial" | string;
  registryDetail: string;
  reports: DiagnosticReport[];
};


export type PrivacyPreferences = {
  diagnosticTelemetryEnabled: boolean;
};
