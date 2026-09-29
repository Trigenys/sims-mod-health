import type {
  GameContentInstallation,
  GameContentSnapshot
} from "../game-content/gameContent.types";

export type UserInstallationCandidate = {
  root: string;
  modsRoot: string;
  source: "knownDocuments" | "oneDriveFallback" | "manual" | string;
  modsAvailable: boolean;
  version:
    | { status: "available"; version: { normalized: string } }
    | { status: "missing" }
    | { status: "invalid"; reason: string };
};

export type ManualUserInspection =
  | { status: "available"; installation: UserInstallationCandidate }
  | { status: "unavailable"; reason: string };

export type SimsSetupSnapshot = {
  gameInventory: GameContentSnapshot;
  userInstallations: UserInstallationCandidate[];
};

export type SimsSetupSelection = {
  game: GameContentInstallation | null;
  user: UserInstallationCandidate | null;
};
