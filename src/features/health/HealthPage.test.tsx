import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HealthPage } from "./HealthPage";
import type { OverviewGateway } from "../overview/overview.gateway";
import type { OverviewSnapshot } from "../overview/overview.types";

const snapshot: OverviewSnapshot = {
  hasInstallation: true,
  gameVersion: "1.128.90",
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
  subscribeProgress: vi.fn().mockResolvedValue(() => undefined)
};

describe("HealthPage", () => {
  it("consolidates findings and filters them into Health subviews", async () => {
    render(<HealthPage gateway={gateway} />);

    expect(await screen.findByRole("heading", { name: "Review what needs attention." })).toBeVisible();
    expect(screen.getByText("A")).toBeVisible();
    expect(screen.getByText("B")).toBeVisible();
    expect(screen.getByText("C")).toBeVisible();

    fireEvent.click(screen.getByRole("button", { name: /Updates/ }));
    expect(screen.getByText("A")).toBeVisible();
    expect(screen.queryByText("B")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Conflicts/ }));
    expect(screen.getByText("B")).toBeVisible();
    expect(screen.queryByText("A")).not.toBeInTheDocument();
  });

  it("treats recovery as a Health subview rather than primary navigation", async () => {
    render(<HealthPage gateway={gateway} initialTab="recovery" />);

    expect(await screen.findByText("Restore points belong to actions, not a separate backup product.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Recovery" })).toHaveAttribute("aria-current", "page");
  });
});
