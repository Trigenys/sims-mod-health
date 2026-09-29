import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { GameContentInstallation, GameContentSnapshot } from "../game-content/gameContent.types";
import type {
  ManualUserInspection,
  SimsSetupSnapshot,
  UserInstallationCandidate
} from "./setup.types";

export type SetupGateway = {
  detect: () => Promise<SimsSetupSnapshot>;
  chooseGameFolder: (title: string) => Promise<GameContentInstallation | null>;
  chooseUserFolder: (title: string) => Promise<UserInstallationCandidate | null>;
  chooseModsFolder: (title: string) => Promise<UserInstallationCandidate | null>;
  scan: (path: string) => Promise<void>;
};

function isVisualHarness() {
  return new URLSearchParams(window.location.search).get("visual") === "setup";
}

const visualUser: UserInstallationCandidate = {
  root: "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4",
  modsRoot: "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods",
  source: "knownDocuments",
  modsAvailable: true,
  version: {
    status: "available",
    version: { normalized: "1.128.90.1030" }
  }
};

const visualGame: GameContentInstallation = {
  installRoot: "C:\\Program Files\\EA Games\\The Sims 4",
  provider: "ea_app",
  providerEvidence: "registry",
  build: {
    version: { normalized: "1.128.90.1030" },
    evidenceKind: "default_ini",
    confidence: "definitive",
    detail: "Version detected locally."
  },
  packs: [
    {
      packCode: "EP01",
      packKind: "expansion",
      localState: "installed",
      sizeBytes: 1,
      markerCount: 1,
      observedAt: "visual"
    }
  ],
  observedAt: "visual"
};

async function pickDirectory(title: string) {
  const selected = await open({
    directory: true,
    multiple: false,
    title
  });
  return Array.isArray(selected) ? selected[0] ?? null : selected;
}

async function inspectUserPath(path: string): Promise<UserInstallationCandidate> {
  const result = await invoke<ManualUserInspection>("inspect_sims_installation", { path });
  if (result.status === "unavailable") {
    throw new Error(result.reason);
  }
  return result.installation;
}

export const setupGateway: SetupGateway = {
  async detect() {
    if (isVisualHarness()) {
      return {
        gameInventory: { installations: [visualGame] },
        userInstallations: [visualUser]
      };
    }

    const [gameInventory, userInstallations] = await Promise.all([
      invoke<GameContentSnapshot>("refresh_game_content_inventory").catch(() => ({
        installations: []
      })),
      invoke<UserInstallationCandidate[]>("discover_sims_installations").catch(() => [])
    ]);

    return { gameInventory, userInstallations };
  },

  async chooseGameFolder(title) {
    if (isVisualHarness()) return visualGame;
    const path = await pickDirectory(title);
    if (!path) return null;
    return invoke<GameContentInstallation>("select_game_content_installation", { path });
  },

  async chooseUserFolder(title) {
    if (isVisualHarness()) return visualUser;
    const path = await pickDirectory(title);
    if (!path) return null;
    return inspectUserPath(path);
  },

  async chooseModsFolder(title) {
    if (isVisualHarness()) return visualUser;
    const path = await pickDirectory(title);
    if (!path) return null;
    return inspectUserPath(path);
  },

  async scan(path) {
    if (isVisualHarness()) return;
    await invoke("scan_sims_mods", { path, mode: "full" });
  }
};
