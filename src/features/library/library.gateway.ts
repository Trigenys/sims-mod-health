import { invoke } from "@tauri-apps/api/core";
import {
  findLibraryItem,
  libraryFixture,
  type LibraryItem
} from "./library.fixture";

export type LibrarySnapshot = {
  hasInstallation: boolean;
  gameVersion: string | null;
  modsRoot: string | null;
  indexedCount: number;
  items: LibraryItem[];
};

export type LibraryGateway = {
  load: () => Promise<LibrarySnapshot>;
};

type LocalLibraryItem = {
  id: number;
  relativePath: string;
  fileKind: string;
  enabled: boolean;
  parseStatus: string;
  sizeBytes: number | null;
  embeddedVersion: string | null;
  creatorHint: string | null;
};

type LocalLibrarySnapshot = {
  hasInstallation: boolean;
  gameVersion: string | null;
  modsRoot: string | null;
  indexedCount: number;
  items: LocalLibraryItem[];
};

function isVisualHarness() {
  const params = new URLSearchParams(window.location.search);
  const visual = params.get("visual");
  return visual === "library" || visual === "detail";
}

function visualSnapshot(): LibrarySnapshot {
  const items = libraryFixture.map((item) => ({
    ...item,
    gameVersion: "1.128.90",
    libraryCount: 324,
    modsRoot: "C:/Users/Player/Documents/Electronic Arts/The Sims 4/Mods"
  }));

  return {
    hasInstallation: true,
    gameVersion: "1.128.90",
    modsRoot: "C:/Users/Player/Documents/Electronic Arts/The Sims 4/Mods",
    indexedCount: 324,
    items
  };
}

function emptySnapshot(): LibrarySnapshot {
  return {
    hasInstallation: false,
    gameVersion: null,
    modsRoot: null,
    indexedCount: 0,
    items: []
  };
}

function basename(path: string) {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}

function fileCategory(kind: string) {
  return kind === "ts4script" ? "Script mod" : "Package / CC";
}

function fileKind(kind: string): LibraryItem["kind"] {
  return kind === "ts4script" ? "Script mod" : "Package only";
}

function toLibraryItem(
  item: LocalLibraryItem,
  snapshot: Pick<LocalLibrarySnapshot, "gameVersion" | "modsRoot" | "indexedCount">
): LibraryItem {
  const filename = basename(item.relativePath);
  const parseNeedsReview = item.parseStatus === "malformed" || item.parseStatus === "error";

  return {
    id: "local-" + item.id,
    canonicalName: filename,
    filenameAliases: [item.relativePath, filename],
    creator: item.creatorHint?.trim() || "Unknown creator",
    category: fileCategory(item.fileKind),
    installedVersion: item.embeddedVersion,
    latestVersion: null,
    status: "Unknown",
    tone: parseNeedsReview ? "warning" : "muted",
    source: "Local only",
    kind: fileKind(item.fileKind),
    identified: false,
    confidence: "unresolved",
    enabled: item.enabled,
    dependencyCount: 0,
    whatItDoes:
      "This is a real file from the scanned Mods folder. Sims Mod Health has not resolved a canonical mod identity for it yet.",
    dependencies: [],
    localFiles: [item.relativePath],
    evidence: [
      {
        type: "fact",
        title: "Scanned local file",
        detail:
          "The desktop scanner indexed this file from your selected Mods folder. Parse state: " +
          item.parseStatus +
          "."
      }
    ],
    relatedMods: [],
    gameVersion: snapshot.gameVersion,
    libraryCount: snapshot.indexedCount,
    modsRoot: snapshot.modsRoot
  };
}

export const libraryGateway: LibraryGateway = {
  async load() {
    if (isVisualHarness()) {
      return visualSnapshot();
    }

    try {
      const snapshot = await invoke<LocalLibrarySnapshot>("get_library_snapshot");
      return {
        hasInstallation: snapshot.hasInstallation,
        gameVersion: snapshot.gameVersion,
        modsRoot: snapshot.modsRoot,
        indexedCount: snapshot.indexedCount,
        items: snapshot.items.map((item) => toLibraryItem(item, snapshot))
      };
    } catch {
      return emptySnapshot();
    }
  }
};

export const libraryVisualGateway: LibraryGateway = {
  async load() {
    return visualSnapshot();
  }
};

export { findLibraryItem };
