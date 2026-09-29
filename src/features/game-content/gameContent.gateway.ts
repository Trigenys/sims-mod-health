import { invoke } from "@tauri-apps/api/core";
import {
  gameContentPartialVisualHealth,
  gameContentVisualHealth,
  gameContentVisualInventory,
  providerVisualCapability,
  providerVisualSession
} from "./gameContent.visual";
import type {
  GameContentHealthSnapshot,
  GameContentSnapshot,
  ProviderUpdateCapability,
  ProviderUpdateSession,
  ProviderUpdateVerification
} from "./gameContent.types";

export type GameContentGateway = {
  loadHealth: () => Promise<GameContentHealthSnapshot>;
  refreshInventory: () => Promise<GameContentSnapshot>;
  loadCapability: () => Promise<ProviderUpdateCapability>;
  startProviderUpdate: (
    targetKind: "game" | "pack",
    targetId: string
  ) => Promise<ProviderUpdateSession>;
  verifyProviderUpdate: (sessionId: number) => Promise<ProviderUpdateVerification>;
};

function isVisualHarness() {
  const visual = new URLSearchParams(window.location.search).get("visual");
  return visual?.startsWith("health") === true || visual?.startsWith("overview") === true;
}

function browserHealthFallback(): GameContentHealthSnapshot {
  return {
    manifestState: "missing",
    manifestVersion: null,
    sourceIdentity: null,
    sourceUrl: null,
    detail: "Game/DLC health requires the desktop runtime.",
    game: null,
    packs: []
  };
}

export const gameContentGateway: GameContentGateway = {
  async loadHealth() {
    if (isVisualHarness()) {
      const visual = new URLSearchParams(window.location.search).get("visual");
      return visual === "overview-partial"
        ? gameContentPartialVisualHealth
        : gameContentVisualHealth;
    }
    try {
      return await invoke<GameContentHealthSnapshot>("get_game_content_health");
    } catch {
      return browserHealthFallback();
    }
  },

  async refreshInventory() {
    if (isVisualHarness()) return gameContentVisualInventory;
    try {
      return await invoke<GameContentSnapshot>("refresh_game_content_inventory");
    } catch {
      return { installations: [] };
    }
  },

  async loadCapability() {
    if (isVisualHarness()) return providerVisualCapability;
    try {
      return await invoke<ProviderUpdateCapability>("get_provider_update_capability");
    } catch {
      return {
        provider: "unknown",
        supported: false,
        actionLabel: "Update in your game provider",
        detail: "The update provider could not be resolved automatically."
      };
    }
  },

  async startProviderUpdate(targetKind, targetId) {
    if (isVisualHarness()) return providerVisualSession(targetKind, targetId);
    return invoke<ProviderUpdateSession>("start_game_content_provider_update", {
      targetKind,
      targetId
    });
  },

  async verifyProviderUpdate(sessionId) {
    if (isVisualHarness()) {
      return {
        session: {
          ...providerVisualSession("game", "game"),
          id: sessionId,
          state: "verified",
          detail: "Local evidence verifies that the update target is current."
        },
        gameContentHealth: gameContentVisualHealth
      };
    }

    return invoke<ProviderUpdateVerification>("verify_game_content_provider_update", {
      sessionId
    });
  }
};
