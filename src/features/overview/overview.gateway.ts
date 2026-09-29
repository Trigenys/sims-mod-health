import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { overviewPartialVisualFixture, overviewVisualFixture } from "./overview.visual";
import type { OverviewSnapshot, ScanProgress } from "./overview.types";

export type OverviewGateway = {
  load: () => Promise<OverviewSnapshot>;
  scanCurrent: () => Promise<void>;
  scanSelected: () => Promise<boolean>;
  subscribeProgress: (
    onProgress: (progress: ScanProgress) => void
  ) => Promise<UnlistenFn>;
};

function isVisualHarness() {
  const visual = new URLSearchParams(window.location.search).get("visual");
  return visual?.startsWith("overview") === true || visual?.startsWith("health") === true;
}

function browserEmptySnapshot(): OverviewSnapshot {
  return {
    hasInstallation: false,
    gameVersion: null,
    platform: "windows",
    indexedCount: 0,
    healthScore: null,
    healthScoreExplanation:
      "Health becomes available after an installation has been scanned and resolved against the registry.",
    healthCounts: {
      healthy: 0,
      updates: 0,
      conflicts: 0,
      unknown: 0
    },
    conflictAggregation: {
      exactDuplicateGroupCount: 0,
      potentialConflictGroupCount: 0,
      attentionGroupCount: 0,
      rawOverlapPairCount: 0,
      suppressedDuplicateOverlapPairCount: 0,
      potentialConflictGroups: []
    },
    attentionCount: 0,
    attention: [],
    installation: {
      scriptMods: 0,
      packageFiles: 0,
      unidentified: null,
      exactDuplicates: 0
    },
    registryState: "offline",
    registryDetail:
      "The browser preview has no Tauri runtime. Local scan data is available in the desktop application.",
    scan: {
      scanSessionId: null,
      status: "empty",
      startedAt: null,
      completedAt: null,
      filesSeen: 0,
      filesHashed: 0,
      filesSkipped: 0,
      observations: 0,
      stale: false,
      partial: false
    }
  };
}

export const overviewGateway: OverviewGateway = {
  async load() {
    if (isVisualHarness()) {
      const visual = new URLSearchParams(window.location.search).get("visual");
      return visual === "overview-partial"
        ? overviewPartialVisualFixture
        : overviewVisualFixture;
    }

    try {
      return await invoke<OverviewSnapshot>("get_overview_snapshot");
    } catch {
      return browserEmptySnapshot();
    }
  },

  async scanCurrent() {
    if (isVisualHarness()) {
      return;
    }

    await invoke("scan_current_sims_mods", { mode: "incremental" });
  },

  async scanSelected() {
    if (isVisualHarness()) {
      return false;
    }

    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select your The Sims 4 Mods folder"
    });
    const path = Array.isArray(selected) ? selected[0] : selected;

    if (!path) {
      return false;
    }

    await invoke("scan_sims_mods", { path, mode: "full" });
    return true;
  },

  async subscribeProgress(onProgress) {
    if (isVisualHarness()) {
      return () => undefined;
    }

    try {
      return await listen<ScanProgress>("scanner://progress", (event) => {
        onProgress(event.payload);
      });
    } catch {
      return () => undefined;
    }
  }
};
