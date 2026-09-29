import { invoke } from "@tauri-apps/api/core";
import { discoverBlockedVisualFixture, discoverVisualFixture } from "./discover.visual";
import type { DiscoverySnapshot } from "./discover.types";

export type DiscoverGateway = {
  load: () => Promise<DiscoverySnapshot>;
};

function isVisualHarness() {
  return new URLSearchParams(window.location.search).get("visual")?.startsWith("discover") === true;
}

function browserSnapshot(): DiscoverySnapshot {
  return {
    patchVersion: null,
    state: "offline",
    detail:
      "Recommendations require the desktop scanner and online compatibility data.",
    blocker: "registry",
    prerequisites: {
      gameDetected: false,
      patchKnown: false,
      packsKnown: false,
      installedPackCount: null,
      modsScanned: false,
      installedModFiles: null,
      identifiedMods: null,
      registryAvailable: false
    },
    recommendations: []
  };
}

export const discoverGateway: DiscoverGateway = {
  async load() {
    if (isVisualHarness()) {
      const visual = new URLSearchParams(window.location.search).get("visual");
      return visual === "discover-blocked"
        ? discoverBlockedVisualFixture
        : discoverVisualFixture;
    }

    try {
      return await invoke<DiscoverySnapshot>("get_discovery_recommendations");
    } catch {
      return browserSnapshot();
    }
  }
};
