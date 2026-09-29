export type DiscoveryBlocker =
  | "game"
  | "patch"
  | "packs"
  | "mods_scan"
  | "identified_mods"
  | "registry"
  | "local_state"
  | string;

export type DiscoveryPrerequisites = {
  gameDetected: boolean;
  patchKnown: boolean;
  packsKnown: boolean;
  installedPackCount: number | null;
  modsScanned: boolean;
  installedModFiles: number | null;
  identifiedMods: number | null;
  registryAvailable: boolean | null;
};

export type DiscoveryRecommendation = {
  modId: string;
  releaseId: string;
  name: string;
  creatorName: string;
  categories: string[];
  features: string[];
  score: number;
  reason: {
    becauseModId: string;
    becauseModName: string;
    sharedCategories: string[];
    sharedFeatures: string[];
    explanation: string;
  };
};

export type DiscoverySnapshot = {
  patchVersion: string | null;
  state: "ready" | "blocked" | "offline" | "partial" | string;
  detail: string;
  blocker: DiscoveryBlocker | null;
  prerequisites: DiscoveryPrerequisites;
  recommendations: DiscoveryRecommendation[];
};
