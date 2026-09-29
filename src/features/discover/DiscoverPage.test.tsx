import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DiscoverPage } from "./DiscoverPage";
import type { DiscoverGateway } from "./discover.gateway";
import type { DiscoverySnapshot } from "./discover.types";

const snapshot: DiscoverySnapshot = {
  patchVersion: "1.128.90",
  state: "ready",
  detail: "Recommendations are ready for the current game and installed mods.",
  blocker: null,
  prerequisites: {
    gameDetected: true,
    patchKnown: true,
    packsKnown: true,
    installedPackCount: 2,
    modsScanned: true,
    installedModFiles: 14,
    identifiedMods: 4,
    registryAvailable: true
  },
  recommendations: [
    {
      modId: "candidate",
      releaseId: "release",
      name: "Candidate Mod",
      creatorName: "Creator",
      categories: ["relationships"],
      features: ["relationship-management"],
      score: 14,
      reason: {
        becauseModId: "installed",
        becauseModName: "Installed Mod",
        sharedCategories: ["relationships"],
        sharedFeatures: ["relationship-management"],
        explanation: "Because you use Installed Mod: Candidate Mod shares relationship-management."
      }
    }
  ]
};

const gateway: DiscoverGateway = {
  load: vi.fn().mockResolvedValue(snapshot)
};

describe("DiscoverPage", () => {
  it("renders explainable recommendations", async () => {
    render(<DiscoverPage gateway={gateway} />);

    expect(await screen.findByText("Candidate Mod")).toBeVisible();
    expect(screen.getByText("Because you use Installed Mod, this mod shares features like Relationship Management and categories like Relationships.")).toBeVisible();
    expect(screen.getByText("Match score 14")).toBeVisible();
  });

  it("shows the exact missing prerequisite and routes the player to Settings", async () => {
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
    const blockedGateway: DiscoverGateway = {
      load: vi.fn().mockResolvedValue(blocked)
    };
    const openSettings = vi.fn();

    render(
      <DiscoverPage
        gateway={blockedGateway}
        onOpenSettings={openSettings}
      />
    );

    expect(
      await screen.findByText("Connect your game before we recommend mods")
    ).toBeVisible();
    expect(screen.getByText("Game not detected")).toBeVisible();
    expect(screen.getByText("2407 mod files scanned")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Review game folders" }));
    expect(openSettings).toHaveBeenCalledTimes(1);
  });

  it("retries online recommendation data without sending the player elsewhere", async () => {
    const offline: DiscoverySnapshot = {
      ...snapshot,
      state: "offline",
      blocker: "registry",
      prerequisites: {
        ...snapshot.prerequisites,
        registryAvailable: false
      },
      recommendations: []
    };
    const retryGateway: DiscoverGateway = {
      load: vi
        .fn()
        .mockResolvedValueOnce(offline)
        .mockResolvedValueOnce(snapshot)
    };

    render(<DiscoverPage gateway={retryGateway} />);

    expect(
      await screen.findByText("Online recommendation data is unavailable")
    ).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    await waitFor(() => expect(retryGateway.load).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("Candidate Mod")).toBeVisible();
  });

  it("distinguishes a ready setup with zero safe candidates from a blocked feature", async () => {
    const readyEmpty: DiscoverySnapshot = {
      ...snapshot,
      recommendations: []
    };
    const emptyGateway: DiscoverGateway = {
      load: vi.fn().mockResolvedValue(readyEmpty)
    };

    render(<DiscoverPage gateway={emptyGateway} />);

    expect(
      await screen.findByText("Your setup is ready, but there is nothing safe to suggest right now")
    ).toBeVisible();
    expect(screen.getByText("Everything needed for recommendations is ready")).toBeVisible();
    expect(screen.queryByText("Recommendations will appear when the missing steps are complete")).not.toBeInTheDocument();
  });

});
