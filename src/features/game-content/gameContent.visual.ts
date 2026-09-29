import type {
  GameContentHealthSnapshot,
  GameContentSnapshot,
  ProviderUpdateCapability,
  ProviderUpdateSession
} from "./gameContent.types";

export const gameContentVisualHealth: GameContentHealthSnapshot = {
  manifestState: "fresh",
  manifestVersion: "2026.09.28",
  sourceIdentity: "Sims Mod Health Registry",
  sourceUrl: null,
  detail: "Game and pack metadata is current.",
  game: {
    kind: "game",
    targetId: "game",
    state: "update_available",
    disputed: false,
    manifestStale: false,
    currentVersion: "1.127.80.1020",
    requiredVersion: "1.128.90.1030",
    reason: "Game build 1.128.90.1030 is newer than installed 1.127.80.1020.",
    evidence: [
      {
        source: "local-default-ini",
        sourceUrl: null,
        detail: "Installed game version 1.127.80.1020 was read locally."
      },
      {
        source: "Sims Mod Health Registry",
        sourceUrl: null,
        detail: "Manifest latest game build is 1.128.90.1030."
      }
    ]
  },
  packs: [
    {
      kind: "pack",
      targetId: "EP01",
      state: "current",
      disputed: false,
      manifestStale: false,
      currentVersion: "1.127.80.1020",
      requiredVersion: "1.50.0.0",
      reason: "Installed game satisfies the pack's known compatibility requirement.",
      evidence: [
        {
          source: "Sims Mod Health Registry",
          sourceUrl: null,
          detail: "Minimum game build: 1.50.0.0."
        }
      ]
    },
    {
      kind: "pack",
      targetId: "EP17",
      state: "game_update_required",
      disputed: false,
      manifestStale: false,
      currentVersion: "1.127.80.1020",
      requiredVersion: "1.128.90.1030",
      reason: "EP17 requires game build 1.128.90.1030 or newer; installed build is 1.127.80.1020.",
      evidence: [
        {
          source: "Sims Mod Health Registry",
          sourceUrl: null,
          detail: "Minimum game build: 1.128.90.1030."
        }
      ]
    },
    ...Array.from({ length: 15 }, (_, index) => ({
      kind: "pack" as const,
      targetId: "SP" + String(index + 1).padStart(2, "0"),
      state: "current" as const,
      disputed: false,
      manifestStale: false,
      currentVersion: "1.127.80.1020",
      requiredVersion: "1.50.0.0",
      reason: "Installed game satisfies the pack's known compatibility requirement.",
      evidence: []
    }))
  ]
};

export const gameContentVisualInventory: GameContentSnapshot = {
  installations: [
    {
      installRoot: "C:\\Program Files\\EA Games\\The Sims 4",
      provider: "ea_app",
      providerEvidence: "registry",
      build: {
        version: { normalized: "1.127.80.1020" },
        evidenceKind: "default_ini",
        confidence: "definitive",
        detail: "Version parsed from Game/Bin/Default.ini."
      },
      packs: gameContentVisualHealth.packs.map((pack) => ({
        packCode: pack.targetId,
        packKind: pack.targetId.startsWith("EP") ? "expansion" : "stuff_or_kit",
        localState: "installed",
        sizeBytes: 1024,
        markerCount: 8,
        observedAt: "2026-09-28T10:00:00Z"
      })),
      observedAt: "2026-09-28T10:00:00Z"
    }
  ]
};

export const providerVisualCapability: ProviderUpdateCapability = {
  provider: "ea_app",
  supported: true,
  actionLabel: "Open EA app to update",
  detail: "EA app is available. Sims Mod Health will open the official client and wait for local verification."
};

export function providerVisualSession(
  targetKind: "game" | "pack",
  targetId: string
): ProviderUpdateSession {
  return {
    id: 76,
    provider: "ea_app",
    targetKind,
    targetId,
    baselineGameVersion: "1.127.80.1020",
    state: "awaiting_rescan",
    detail: "Waiting for a local rescan to verify the resulting game and pack state.",
    providerExecutableName: "EADesktop.exe",
    createdAt: "2026-09-28T10:00:00Z",
    updatedAt: "2026-09-28T10:01:00Z",
    completedAt: null,
    lastError: null
  };
}


export const gameContentPartialVisualHealth: GameContentHealthSnapshot = {
  manifestState: "missing",
  manifestVersion: null,
  sourceIdentity: null,
  sourceUrl: null,
  detail: "No local The Sims 4 program installation has been inventoried yet.",
  game: null,
  packs: []
};
