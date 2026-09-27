import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DiscoverPage } from "./DiscoverPage";
import type { DiscoverGateway } from "./discover.gateway";
import type { DiscoverySnapshot } from "./discover.types";

const snapshot: DiscoverySnapshot = {
  patchVersion: "1.128.90",
  state: "ready",
  detail: "Filtered first.",
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
  it("renders explainable Registry recommendations", async () => {
    render(<DiscoverPage gateway={gateway} />);

    expect(await screen.findByText("Candidate Mod")).toBeVisible();
    expect(screen.getByText("Because you use Installed Mod: Candidate Mod shares relationship-management.")).toBeVisible();
    expect(screen.getByText("Deterministic score 14")).toBeVisible();
  });
});
