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
  state: "ready" | "empty" | "offline" | "partial" | string;
  detail: string;
  recommendations: DiscoveryRecommendation[];
};
