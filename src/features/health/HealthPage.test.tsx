import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HealthPage } from "./HealthPage";
import type { GameContentGateway } from "../game-content/gameContent.gateway";
import {
  gameContentVisualHealth,
  gameContentVisualInventory,
  providerVisualCapability,
  providerVisualSession
} from "../game-content/gameContent.visual";
import type { OverviewGateway } from "../overview/overview.gateway";
import type { OverviewSnapshot } from "../overview/overview.types";

const snapshot: OverviewSnapshot = {
  hasInstallation: true,
  gameVersion: "1.127.80.1020",
  platform: "windows",
  indexedCount: 12,
  healthScore: 82,
  healthScoreExplanation: "Evidence based.",
  healthCounts: {
    healthy: 8,
    updates: 1,
    conflicts: 2,
    unknown: 1
  },
  conflictAggregation: {
    exactDuplicateGroupCount: 1,
    potentialConflictGroupCount: 1,
    attentionGroupCount: 1,
    rawOverlapPairCount: 4,
    suppressedDuplicateOverlapPairCount: 1,
    potentialConflictGroups: [
      {
        classification: "potentialConflictGroup",
        confidence: "low",
        countsTowardAttention: false,
        fileIds: [10, 11, 12],
        relativePaths: [
          "CreatorA/eyes.package",
          "CreatorB/eyes-overlay.package",
          "CreatorC/eyes-default.package"
        ],
        overlapPairCount: 3,
        sharedResourceCount: 7,
        sampleResourceKeys: [
          { resourceType: 3451, group: 0, instance: 42 }
        ],
        sampleOverlapPairs: [
          {
            classification: "potentialConflict",
            leftFileId: 10,
            leftRelativePath: "CreatorA/eyes.package",
            rightFileId: 11,
            rightRelativePath: "CreatorB/eyes-overlay.package",
            sharedResourceCount: 3,
            sampleResourceKeys: [
              { resourceType: 3451, group: 0, instance: 42 }
            ]
          }
        ]
      }
    ]
  },
  attentionCount: 3,
  attention: [
    {
      name: "A",
      creator: "Registry health",
      detail: "Update available.",
      badge: "Update",
      tone: "update"
    },
    {
      name: "B",
      creator: "Local scan",
      detail: "2 exact copies share a SHA-256 fingerprint.",
      badge: "Duplicate",
      tone: "warning"
    },
    {
      name: "C",
      creator: "Registry health",
      detail: "No current evidence.",
      badge: "Unknown",
      tone: "muted"
    }
  ],
  installation: {
    scriptMods: 3,
    packageFiles: 9,
    unidentified: 1,
    exactDuplicates: 1
  },
  registryState: "ready",
  registryDetail: "Current",
  scan: {
    scanSessionId: 1,
    status: "completed",
    startedAt: null,
    completedAt: null,
    filesSeen: 12,
    filesHashed: 2,
    filesSkipped: 10,
    observations: 0,
    stale: false,
    partial: false
  }
};

const gateway: OverviewGateway = {
  load: vi.fn().mockResolvedValue(snapshot),
  scanCurrent: vi.fn().mockResolvedValue(undefined),
  scanSelected: vi.fn().mockResolvedValue(false),
  subscribeProgress: vi.fn().mockResolvedValue(() => undefined)
};

function contentGateway(): GameContentGateway {
  return {
    loadHealth: vi.fn().mockResolvedValue(gameContentVisualHealth),
    refreshInventory: vi.fn().mockResolvedValue(gameContentVisualInventory),
    loadCapability: vi.fn().mockResolvedValue(providerVisualCapability),
    startProviderUpdate: vi.fn().mockImplementation((kind, id) =>
      Promise.resolve(providerVisualSession(kind, id))
    ),
    verifyProviderUpdate: vi.fn().mockImplementation((sessionId) =>
      Promise.resolve({
        session: {
          ...providerVisualSession("pack", "EP17"),
          id: sessionId,
          state: "verified"
        },
        gameContentHealth: gameContentVisualHealth
      })
    )
  };
}

describe("HealthPage", () => {
  it("combines Game, Pack and Mod findings and filters one update queue", async () => {
    render(<HealthPage gateway={gateway} contentGateway={contentGateway()} />);

    expect(await screen.findByRole("heading", { name: "Review what needs attention." })).toBeVisible();
    expect(screen.getByText("The Sims 4")).toBeVisible();
    expect(screen.getByText("EP17")).toBeVisible();
    expect(screen.getByText("A")).toBeVisible();
    expect(screen.getByText("B")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: /Updates/ }));
    expect(screen.getByText("The Sims 4")).toBeVisible();
    expect(screen.getByText("EP17")).toBeVisible();
    expect(screen.getByText("A")).toBeVisible();
    expect(screen.queryByText("B")).not.toBeInTheDocument();
    expect(screen.getByText("16 packs current")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: "Mods" }));
    expect(screen.getByText("A")).toBeVisible();
    expect(screen.queryByText("EP17")).not.toBeInTheDocument();
  });

  it("opens pack evidence progressively and keeps provider action contextual", async () => {
    const content = contentGateway();
    render(<HealthPage gateway={gateway} contentGateway={content} initialTab="updates" />);

    const pack = await screen.findByText("EP17");
    const card = pack.closest("article");
    expect(card).not.toBeNull();
    fireEvent.click(within(card as HTMLElement).getByRole("button", { name: "Review details" }));

    const drawer = screen.getByRole("dialog", { name: "EP17" });
    expect(within(drawer).getByText("Minimum game build")).toBeVisible();
    expect(within(drawer).getByText("1.128.90.1030")).toBeVisible();

    fireEvent.click(within(drawer).getByRole("button", { name: "Open EA app to update" }));
    expect(content.startProviderUpdate).toHaveBeenCalledWith("pack", "EP17");
  });

  it("groups low-confidence DBPF interactions separately and exposes sampled evidence", async () => {
    render(
      <HealthPage
        gateway={gateway}
        contentGateway={contentGateway()}
        initialTab="conflicts"
      />
    );

    const potential = await screen.findByText("Potential interaction across 3 files");
    const card = potential.closest("article");
    expect(card).not.toBeNull();
    expect(within(card as HTMLElement).getByText("Low confidence")).toBeVisible();

    fireEvent.click(within(card as HTMLElement).getByRole("button", { name: "Review evidence" }));

    const drawer = screen.getByRole("dialog", { name: "Potential interaction evidence" });
    expect(within(drawer).getByText("Raw pair observations")).toBeVisible();
    expect(within(drawer).getByText("CreatorA/eyes.package")).toBeVisible();
    expect(within(drawer).getByText("CreatorB/eyes-overlay.package")).toBeVisible();
    expect(within(drawer).getByText("3 shared resource references")).toBeVisible();
  });

  it("treats recovery as a Health subview rather than primary navigation", async () => {
    render(
      <HealthPage
        gateway={gateway}
        contentGateway={contentGateway()}
        initialTab="recovery"
      />
    );

    expect(await screen.findByText("Restore points belong to actions, not a separate backup product.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Recovery" })).toHaveAttribute("aria-current", "page");
  });
});
