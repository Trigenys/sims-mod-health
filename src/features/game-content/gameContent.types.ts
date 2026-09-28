export type GameContentHealthState =
  | "current"
  | "update_available"
  | "game_update_required"
  | "metadata_stale"
  | "local_integrity_uncertain"
  | "unknown";

export type GameContentFindingKind = "game" | "pack";

export type GameContentEvidence = {
  source: string;
  sourceUrl: string | null;
  detail: string;
};

export type GameContentHealthFinding = {
  kind: GameContentFindingKind;
  targetId: string;
  state: GameContentHealthState;
  disputed: boolean;
  manifestStale: boolean;
  currentVersion: string | null;
  requiredVersion: string | null;
  reason: string;
  evidence: GameContentEvidence[];
};

export type GameContentHealthSnapshot = {
  manifestState: "fresh" | "cached_stale" | "missing";
  manifestVersion: string | null;
  sourceIdentity: string | null;
  sourceUrl: string | null;
  detail: string;
  game: GameContentHealthFinding | null;
  packs: GameContentHealthFinding[];
};

export type GameContentInstallation = {
  installRoot: string;
  provider: "ea_app" | "steam" | "unknown" | string;
  providerEvidence: string;
  build: {
    version: { normalized: string } | null;
    evidenceKind: string;
    confidence: string;
    detail: string;
  };
  packs: Array<{
    packCode: string;
    packKind: string;
    localState: string;
    sizeBytes: number | null;
    markerCount: number;
    observedAt: string;
  }>;
  observedAt: string;
};

export type GameContentSnapshot = {
  installations: GameContentInstallation[];
};

export type ProviderUpdateCapability = {
  provider: string;
  supported: boolean;
  actionLabel: string;
  detail: string;
};

export type ProviderUpdateSession = {
  id: number;
  provider: string;
  targetKind: "game" | "pack";
  targetId: string;
  baselineGameVersion: string | null;
  state:
    | "detected"
    | "action_required"
    | "provider_opened"
    | "awaiting_rescan"
    | "verified"
    | "still_outdated"
    | "unknown"
    | "failed";
  detail: string;
  providerExecutableName: string | null;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  lastError: string | null;
};

export type ProviderUpdateVerification = {
  session: ProviderUpdateSession;
  gameContentHealth: GameContentHealthSnapshot;
};
