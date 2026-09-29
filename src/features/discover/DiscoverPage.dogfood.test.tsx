import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DiscoverPage } from "./DiscoverPage";
import type { DiscoverGateway } from "./discover.gateway";
import type { DiscoverySnapshot } from "./discover.types";

function gateway(snapshot: DiscoverySnapshot): DiscoverGateway {
  return {
    load: vi.fn().mockResolvedValue(snapshot)
  };
}

describe("real-installation Discover dogfood regressions", () => {
  it("explains that a scanned Mods library is still blocked when the game path is missing", async () => {
    const blocked: DiscoverySnapshot = {
      patchVersion: null,
      state: "blocked",
      detail: "The Sims 4 program installation has not been detected yet.",
      blocker: "game",
      prerequisites: {
        gameDetected: false,
        patchKnown: false,
        packsKnown: false,
        installedPackCount: null,
        modsScanned: true,
        installedModFiles: 2407,
        identifiedMods: null,
        registryAvailable: null
      },
      recommendations: []
    };

    render(<DiscoverPage gateway={gateway(blocked)} onOpenSettings={vi.fn()} />);

    expect(await screen.findByText("Connect your game before we recommend mods")).toBeVisible();
    expect(screen.getByText("Game not detected")).toBeVisible();
    expect(screen.getByText("2407 mod files scanned")).toBeVisible();
    expect(screen.getByRole("button", { name: "Review game folders" })).toBeEnabled();
  });

  it("shows recommendations only after game, packs, scan, identities and online data are ready", async () => {
    const ready: DiscoverySnapshot = {
      patchVersion: "1.128.90.1030",
      state: "ready",
      detail: "Recommendations are ready for the current game and installed mods.",
      blocker: null,
      prerequisites: {
        gameDetected: true,
        patchKnown: true,
        packsKnown: true,
        installedPackCount: 17,
        modsScanned: true,
        installedModFiles: 2407,
        identifiedMods: 318,
        registryAvailable: true
      },
      recommendations: [
        {
          modId: "candidate",
          releaseId: "candidate-r1",
          name: "Candidate Mod",
          creatorName: "Creator",
          categories: ["gameplay"],
          features: ["quality-of-life"],
          score: 18,
          reason: {
            becauseModId: "installed",
            becauseModName: "Installed Mod",
            sharedCategories: ["gameplay"],
            sharedFeatures: ["quality-of-life"],
            explanation: "backend wording is intentionally not rendered directly"
          }
        }
      ]
    };

    render(<DiscoverPage gateway={gateway(ready)} />);

    expect(await screen.findByText("Everything needed for recommendations is ready")).toBeVisible();
    expect(screen.getByText("17 installed packs found")).toBeVisible();
    expect(screen.getByText("318 installed mods identified")).toBeVisible();
    expect(screen.getByText("2407 mod files scanned")).toBeVisible();
    expect(screen.getByText("Candidate Mod")).toBeVisible();
    expect(
      screen.getByText(
        "Because you use Installed Mod, this mod shares features like Quality Of Life and categories like Gameplay."
      )
    ).toBeVisible();
    expect(screen.queryByText("backend wording is intentionally not rendered directly")).not.toBeInTheDocument();
  });
});
