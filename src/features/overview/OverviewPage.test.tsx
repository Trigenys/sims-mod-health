import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { OverviewPage } from "./OverviewPage";
import type { GameContentGateway } from "../game-content/gameContent.gateway";
import {
  gameContentVisualHealth,
  gameContentVisualInventory,
  providerVisualCapability,
  providerVisualSession
} from "../game-content/gameContent.visual";
import type { OverviewGateway } from "./overview.gateway";
import type { OverviewSnapshot } from "./overview.types";

const snapshot: OverviewSnapshot = {
  hasInstallation: true,
  gameVersion: "1.128.90",
  platform: "windows",
  indexedCount: 12,
  healthScore: 80,
  healthScoreExplanation: "Verified compatible releases divided by resolved releases plus unresolved files.",
  healthCounts: {
    healthy: 7,
    updates: 1,
    conflicts: 2,
    unknown: 2
  },
  attentionCount: 2,
  attention: [
    {
      name: "A.package",
      creator: "Local scan",
      detail: "2 exact copies share the same SHA-256 fingerprint.",
      badge: "Duplicate",
      tone: "warning"
    }
  ],
  installation: {
    scriptMods: 3,
    packageFiles: 9,
    unidentified: 2,
    exactDuplicates: 1
  },
  registryState: "ready",
  registryDetail: "Current",
  scan: {
    scanSessionId: 10,
    status: "completed",
    startedAt: "2026-09-26T20:00:00Z",
    completedAt: "2026-09-26T20:01:00Z",
    filesSeen: 12,
    filesHashed: 2,
    filesSkipped: 10,
    observations: 0,
    stale: false,
    partial: false
  }
};

const contentGateway: GameContentGateway = {
  loadHealth: vi.fn().mockResolvedValue(gameContentVisualHealth),
  refreshInventory: vi.fn().mockResolvedValue(gameContentVisualInventory),
  loadCapability: vi.fn().mockResolvedValue(providerVisualCapability),
  startProviderUpdate: vi.fn().mockImplementation((kind, id) =>
    Promise.resolve(providerVisualSession(kind, id))
  ),
  verifyProviderUpdate: vi.fn()
};

function gateway(value: OverviewSnapshot): OverviewGateway {
  return {
    load: vi.fn().mockResolvedValue(value),
    scanCurrent: vi.fn().mockResolvedValue(undefined),
    scanSelected: vi.fn().mockResolvedValue(false),
    subscribeProgress: vi.fn().mockResolvedValue(() => undefined)
  };
}

describe("OverviewPage", () => {
  it("renders current scan and health-engine counts instead of design-target constants", async () => {
    render(<OverviewPage gateway={gateway(snapshot)} contentGateway={contentGateway} />);

    expect(await screen.findByRole("heading", { name: "A few things are worth checking." })).toBeVisible();
    expect(screen.getByLabelText("Overall health 80 percent")).toBeVisible();
    expect(screen.getByText("12 items indexed")).toBeVisible();
    const gameSummary = screen.getByLabelText("Sims 4 installation summary");
    expect(within(gameSummary).getByText("Installed packs")).toBeVisible();
    expect(within(gameSummary).getByText("17")).toBeVisible();
    expect(within(gameSummary).getByText("Game / pack attention")).toBeVisible();
    expect(within(gameSummary).getByText("2")).toBeVisible();
    expect(screen.getByText("3")).toBeVisible();
    expect(screen.getByText("9")).toBeVisible();
    expect(screen.getByText("Verified compatible releases divided by resolved releases plus unresolved files.")).not.toBeVisible();

    fireEvent.click(screen.getByText("How this score works"));
    expect(screen.getByText("Verified compatible releases divided by resolved releases plus unresolved files.")).toBeVisible();
  });

  it("keeps local scan data visible when the registry is offline", async () => {
    const offline: OverviewSnapshot = {
      ...snapshot,
      healthScore: null,
      registryState: "offline",
      registryDetail: "registry transport error",
      installation: {
        ...snapshot.installation,
        unidentified: null
      }
    };

    render(<OverviewPage gateway={gateway(offline)} contentGateway={contentGateway} />);

    expect(await screen.findByText("Online checks are temporarily unavailable")).toBeVisible();
    expect(screen.getByText("12 items indexed")).toBeVisible();
    expect(screen.getByText("Exact duplicate groups")).toBeVisible();
    expect(screen.getByLabelText("Overall health unavailable")).toBeVisible();
  });

  it("runs an incremental scan and refreshes the snapshot", async () => {
    const scanCurrent = vi.fn().mockResolvedValue(undefined);
    const load = vi.fn().mockResolvedValue(snapshot);
    const fakeGateway: OverviewGateway = {
      load,
      scanCurrent,
      scanSelected: vi.fn().mockResolvedValue(false),
      subscribeProgress: vi.fn().mockResolvedValue(() => undefined)
    };

    render(<OverviewPage gateway={fakeGateway} contentGateway={contentGateway} />);

    const button = await screen.findByRole("button", { name: "Scan now" });
    fireEvent.click(button);

    await waitFor(() => expect(scanCurrent).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(load).toHaveBeenCalledTimes(2));
  });

  it("shows the complete Sims setup and keeps topbar scanning disabled before first scan", async () => {
    const empty: OverviewSnapshot = {
      ...snapshot,
      hasInstallation: false,
      gameVersion: null,
      indexedCount: 0,
      healthScore: null,
      healthCounts: { healthy: 0, updates: 0, conflicts: 0, unknown: 0 },
      attentionCount: 0,
      attention: [],
      installation: {
        scriptMods: 0,
        packageFiles: 0,
        unidentified: null,
        exactDuplicates: 0
      },
      scan: {
        scanSessionId: null,
        status: "empty",
        startedAt: null,
        completedAt: null,
        filesSeen: 0,
        filesHashed: 0,
        filesSkipped: 0,
        observations: 0,
        stale: false,
        partial: false
      }
    };
    const fakeGateway: OverviewGateway = {
      load: vi.fn().mockResolvedValue(empty),
      scanCurrent: vi.fn().mockResolvedValue(undefined),
      scanSelected: vi.fn().mockResolvedValue(false),
      subscribeProgress: vi.fn().mockResolvedValue(() => undefined)
    };

    render(<OverviewPage gateway={fakeGateway} contentGateway={contentGateway} />);

    expect(await screen.findByRole("heading", { name: "Connect your Sims 4 installation" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Scan now" })).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Choose Mods folder and scan" })).not.toBeInTheDocument();
  });

});
