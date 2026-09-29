import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { I18nProvider } from "../../i18n/i18n";
import type { GameContentInstallation } from "../game-content/gameContent.types";
import { SimsSetupPanel } from "./SimsSetupPanel";
import type { SetupGateway } from "./setup.gateway";
import type { UserInstallationCandidate } from "./setup.types";

const userInstallation: UserInstallationCandidate = {
  root: "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4",
  modsRoot: "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods",
  source: "knownDocuments",
  modsAvailable: true,
  version: {
    status: "available",
    version: { normalized: "1.128.90.1030" }
  }
};

function gameInstallation({
  provider = "ea_app",
  installRoot = "D:\\Games\\EA Games\\The Sims 4",
  version = "1.128.90.1030",
  packs = 17
}: {
  provider?: "ea_app" | "steam";
  installRoot?: string;
  version?: string | null;
  packs?: number;
} = {}): GameContentInstallation {
  return {
    installRoot,
    provider,
    providerEvidence: "manual",
    build: {
      version: version ? { normalized: version } : null,
      evidenceKind: version ? "default_ini" : "unknown",
      confidence: version ? "definitive" : "unknown",
      detail: version ? "Version detected locally." : "Version file could not be read."
    },
    packs: Array.from({ length: packs }, (_, index) => ({
      packCode: "EP" + String(index + 1).padStart(2, "0"),
      packKind: "expansion",
      localState: "installed",
      sizeBytes: 1,
      markerCount: 1,
      observedAt: "dogfood"
    })),
    observedAt: "dogfood"
  };
}

function dogfoodGateway({
  game = gameInstallation(),
  user = userInstallation
}: {
  game?: GameContentInstallation | null;
  user?: UserInstallationCandidate | null;
} = {}): SetupGateway {
  return {
    detect: vi.fn().mockResolvedValue({
      gameInventory: { installations: game ? [game] : [] },
      userInstallations: user ? [user] : []
    }),
    chooseGameFolder: vi.fn().mockResolvedValue(gameInstallation()),
    chooseUserFolder: vi.fn().mockResolvedValue(userInstallation),
    chooseModsFolder: vi.fn().mockResolvedValue(userInstallation),
    scan: vi.fn().mockResolvedValue(undefined)
  };
}

describe("real-installation first-run dogfood regressions", () => {
  afterEach(() => {
    window.localStorage.removeItem("sims-mod-health.locale");
  });

  it("keeps scanning locked when the Mods folder exists but the program game path is missing", async () => {
    render(
      <SimsSetupPanel
        gateway={dogfoodGateway({ game: null, user: userInstallation })}
        onScanComplete={vi.fn()}
      />
    );

    expect(await screen.findByText("Mods folder ready to scan.")).toBeVisible();
    expect(screen.getByText("We could not find the installed game automatically.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Scan my mods" })).toBeDisabled();
  });

  it("unlocks the first scan when game, user folder, Mods and version are all detected", async () => {
    render(
      <SimsSetupPanel
        gateway={dogfoodGateway()}
        onScanComplete={vi.fn()}
      />
    );

    expect(await screen.findByText("Version 1.128.90.1030 detected")).toBeVisible();
    expect(screen.getByText("17 installed packs found")).toBeVisible();
    expect(screen.getByText("Mods folder ready to scan.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Scan my mods" })).toBeEnabled();
  });

  it.each([
    ["EA app", "ea_app", "D:\\Custom\\EA Games\\The Sims 4"],
    ["Steam", "steam", "E:\\SteamLibrary\\steamapps\\common\\The Sims 4"]
  ] as const)("shows a custom %s install path as a valid detected game", async (label, provider, installRoot) => {
    render(
      <SimsSetupPanel
        gateway={dogfoodGateway({
          game: gameInstallation({ provider, installRoot })
        })}
        onScanComplete={vi.fn()}
      />
    );

    expect(await screen.findByText(`Found with ${label}`)).toBeVisible();
    expect(screen.getByTitle(installRoot)).toHaveTextContent(installRoot);
    expect(screen.getByRole("button", { name: "Scan my mods" })).toBeEnabled();
  });

  it("does not turn an unreadable game version into a usable setup", async () => {
    render(
      <SimsSetupPanel
        gateway={dogfoodGateway({
          game: gameInstallation({ version: null })
        })}
        onScanComplete={vi.fn()}
      />
    );

    expect(
      await screen.findByText(
        "We found the game, but could not read its version. Choose the correct game folder or detect again."
      )
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "Scan my mods" })).toBeDisabled();
  });

  it("keeps the complete first-run outcome readable in English", async () => {
    render(
      <I18nProvider>
        <SimsSetupPanel
          gateway={dogfoodGateway()}
          onScanComplete={vi.fn()}
        />
      </I18nProvider>
    );

    expect(await screen.findByRole("heading", { name: "Connect your Sims 4 installation" })).toBeVisible();
    expect(screen.getByText("Everything needed for a useful scan is ready.")).toBeVisible();
  });

  it("keeps the complete first-run outcome readable in French", async () => {
    window.localStorage.setItem("sims-mod-health.locale", "fr");

    render(
      <I18nProvider>
        <SimsSetupPanel
          gateway={dogfoodGateway()}
          onScanComplete={vi.fn()}
        />
      </I18nProvider>
    );

    expect(await screen.findByRole("heading", { name: "Connectez votre installation des Sims 4" })).toBeVisible();
    expect(screen.getByText("Tout est prêt pour une analyse utile.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Analyser mes mods" })).toBeEnabled();
  });
});
