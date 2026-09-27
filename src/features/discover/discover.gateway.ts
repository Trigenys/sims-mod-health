import { invoke } from "@tauri-apps/api/core";
import { discoverVisualFixture } from "./discover.visual";
import type { DiscoverySnapshot } from "./discover.types";

export type DiscoverGateway = {
  load: () => Promise<DiscoverySnapshot>;
};

function isVisualHarness() {
  return new URLSearchParams(window.location.search).get("visual") === "discover";
}

function browserSnapshot(): DiscoverySnapshot {
  return {
    patchVersion: null,
    state: "offline",
    detail:
      "Recommendations require the desktop scanner and Registry resolution. Browser preview does not invent recommendation data.",
    recommendations: []
  };
}

export const discoverGateway: DiscoverGateway = {
  async load() {
    if (isVisualHarness()) {
      return discoverVisualFixture;
    }

    try {
      return await invoke<DiscoverySnapshot>("get_discovery_recommendations");
    } catch {
      return browserSnapshot();
    }
  }
};
