import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GameInstallationSummary } from "./GameInstallationSummary";
import type { GameContentHealthSnapshot } from "./gameContent.types";

describe("GameInstallationSummary", () => {
  it("renders unavailable values instead of zero when game evidence is missing", () => {
    const missing: GameContentHealthSnapshot = {
      manifestState: "missing",
      manifestVersion: null,
      sourceIdentity: null,
      sourceUrl: null,
      detail: "missing",
      game: null,
      packs: []
    };

    render(
      <GameInstallationSummary
        health={missing}
        modCount={2407}
        fallbackVersion={null}
      />
    );

    const summary = screen.getByLabelText("Sims 4 installation summary");
    const game = within(summary).getByText("Game build").parentElement;
    const packs = within(summary).getByText("Installed packs").parentElement;
    const attention = within(summary).getByText("Game / pack attention").parentElement;

    expect(within(game as HTMLElement).getByText("Not detected")).toBeVisible();
    expect(within(packs as HTMLElement).getByText("Not detected")).toBeVisible();
    expect(within(attention as HTMLElement).getByText("Not checked")).toBeVisible();
    expect(within(summary).getByText("2407")).toBeVisible();
  });

  it("keeps a real zero pack count when the game is measured", () => {
    const measured: GameContentHealthSnapshot = {
      manifestState: "fresh",
      manifestVersion: "m1",
      sourceIdentity: "registry",
      sourceUrl: null,
      detail: "fresh",
      game: {
        kind: "game",
        targetId: "game",
        state: "current",
        disputed: false,
        manifestStale: false,
        currentVersion: "1.128.90.1030",
        requiredVersion: "1.128.90.1030",
        reason: "current",
        evidence: []
      },
      packs: []
    };

    render(
      <GameInstallationSummary
        health={measured}
        modCount={12}
        fallbackVersion={null}
      />
    );

    const summary = screen.getByLabelText("Sims 4 installation summary");
    const packs = within(summary).getByText("Installed packs").parentElement;
    const attention = within(summary).getByText("Game / pack attention").parentElement;

    expect(within(packs as HTMLElement).getByText("0")).toBeVisible();
    expect(within(attention as HTMLElement).getByText("0")).toBeVisible();
  });
});
