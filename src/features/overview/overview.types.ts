import type { StatusTone } from "../../components/ui/StatusBadge";

export type RegistryState = "ready" | "offline" | "partial";

export type OverviewAttentionItem = {
  name: string;
  creator: string;
  detail: string;
  badge: string;
  tone: StatusTone;
};

export type OverviewSnapshot = {
  hasInstallation: boolean;
  gameVersion: string | null;
  platform: string;
  indexedCount: number;
  healthScore: number | null;
  healthScoreExplanation: string;
  healthCounts: {
    healthy: number;
    updates: number;
    conflicts: number;
    unknown: number;
  };
  attentionCount: number;
  attention: OverviewAttentionItem[];
  installation: {
    scriptMods: number;
    packageFiles: number;
    unidentified: number | null;
    exactDuplicates: number;
  };
  registryState: RegistryState;
  registryDetail: string;
  scan: {
    scanSessionId: number | null;
    status: "empty" | "running" | "completed" | "failed" | "cancelled" | string;
    startedAt: string | null;
    completedAt: string | null;
    filesSeen: number;
    filesHashed: number;
    filesSkipped: number;
    observations: number;
    stale: boolean;
    partial: boolean;
  };
};

export type ScanProgress = {
  filesSeen: number;
  filesHashed: number;
  filesSkipped: number;
  observations: number;
};
