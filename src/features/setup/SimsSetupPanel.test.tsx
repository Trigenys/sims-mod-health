import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { I18nProvider } from "../../i18n/i18n";
import type { GameContentInstallation } from "../game-content/gameContent.types";
import { SimsSetupPanel } from "./SimsSetupPanel";
import type { SetupGateway } from "./setup.gateway";
import type { UserInstallationCandidate } from "./setup.types";

const user: UserInstallationCandidate = {
  root: "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4",
  modsRoot: "C:\\Users\\Player\\Documents\\Electronic Arts\\The Sims 4\\Mods",
  source: "knownDocuments",
  modsAvailable: true,
  version: {
    status: "available",
    version: { normalized: "1.128.90.1030" }
  }
};

function game(packCount = 2): GameContentInstallation {
  return {
    installRoot: "C:\\Program Files\\EA Games\\The Sims 4",
    provider: "ea_app",
    providerEvidence: "registry",
    build: {
      version: { normalized: "1.128.90.1030" },
      evidenceKind: "default_ini",
      confidence: "definitive",
      detail: "fixture"
    },
    packs: Array.from({ length: packCount }, (_, index) => ({
      packCode: "EP" + String(index + 1).padStart(2, "0"),
      packKind: "expansion",
      localState: "installed",
      sizeBytes: 1,
      markerCount: 1,
      observedAt: "test"
    })),
    observedAt: "test"
  };
}

function gateway({
  detectedGame = game(),
  detectedUser = user,
  selectedGame = game()
}: {
  detectedGame?: GameContentInstallation | null;
  detectedUser?: UserInstallationCandidate | null;
  selectedGame?: GameContentInstallation;
} = {}): SetupGateway {
  return {
    detect: vi.fn().mockResolvedValue({
      gameInventory: { installations: detectedGame ? [detectedGame] : [] },
      userInstallations: detectedUser ? [detectedUser] : []
    }),
    chooseGameFolder: vi.fn().mockResolvedValue(selectedGame),
    chooseUserFolder: vi.fn().mockResolvedValue(user),
    chooseModsFolder: vi.fn().mockResolvedValue(user),
    scan: vi.fn().mockResolvedValue(undefined)
  };
}

describe("SimsSetupPanel", () => {
  afterEach(() => {
    window.localStorage.removeItem("sims-mod-health.locale");
  });

  it("uses auto-detected game, version, packs and Mods before scanning", async () => {
    const fake = gateway();
    const completed = vi.fn();

    render(<SimsSetupPanel gateway={fake} onScanComplete={completed} />);

    expect(await screen.findByText("Connect your Sims 4 installation")).toBeVisible();
    expect(screen.getByText("Version 1.128.90.1030 detected")).toBeVisible();
    expect(screen.getByText("2 installed packs found")).toBeVisible();
    expect(screen.getByText("Mods folder ready to scan.")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Scan my mods" }));

    await waitFor(() => {
      expect(fake.scan).toHaveBeenCalledWith(user.root);
      expect(completed).toHaveBeenCalledTimes(1);
    });
  });

  it("blocks the scan until a missing game installation is selected", async () => {
    const fake = gateway({ detectedGame: null });
    render(<SimsSetupPanel gateway={fake} onScanComplete={vi.fn()} />);

    const scan = await screen.findByRole("button", { name: "Scan my mods" });
    expect(scan).toBeDisabled();
    expect(screen.getByText("We could not find the installed game automatically.")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Choose game folder" }));

    await waitFor(() => expect(fake.chooseGameFolder).toHaveBeenCalledTimes(1));
    expect(screen.getByText("Version 1.128.90.1030 detected")).toBeVisible();
    expect(scan).toBeEnabled();
  });

  it("treats zero extra packs as a valid base-game setup", async () => {
    const fake = gateway({ detectedGame: game(0) });
    render(<SimsSetupPanel gateway={fake} onScanComplete={vi.fn()} />);

    expect(await screen.findByText("No extra packs were found. Base game only is okay.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Scan my mods" })).toBeEnabled();
  });

  it("renders the first-run flow in French", async () => {
    window.localStorage.setItem("sims-mod-health.locale", "fr");

    render(
      <I18nProvider>
        <SimsSetupPanel gateway={gateway()} onScanComplete={vi.fn()} />
      </I18nProvider>
    );

    expect(await screen.findByText("Connectez votre installation des Sims 4")).toBeVisible();
    expect(screen.getByText("Tout est prêt pour une analyse utile.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Analyser mes mods" })).toBeEnabled();
  });
});
