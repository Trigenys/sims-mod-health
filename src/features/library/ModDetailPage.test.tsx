import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ModDetailPage } from "./ModDetailPage";
import { findLibraryItem } from "./library.fixture";

describe("ModDetailPage", () => {
  it("separates verified facts, inferred identification and community reports", () => {
    render(
      <ModDetailPage
        item={findLibraryItem("rpo")}
        onBack={vi.fn()}
      />
    );

    expect(screen.getByText("Verified fact")).toBeVisible();
    expect(screen.getByText("Inferred identification")).toBeVisible();
    expect(screen.getByText("Community report")).toBeVisible();
    expect(screen.getByText("High confidence")).toBeVisible();
    expect(screen.getByRole("heading", { name: "Why the app says this" })).toBeVisible();
  });

  it("shows installed/latest versions, dependencies, files and related mods", () => {
    render(
      <ModDetailPage
        item={findLibraryItem("mccc")}
        onBack={vi.fn()}
      />
    );

    expect(screen.getByText("2026.4.0")).toBeVisible();
    expect(screen.getByText("2026.5.0")).toBeVisible();
    expect(screen.getByText("mc_cmd_center.package")).toBeVisible();
    expect(screen.getByText("No required dependencies")).toBeVisible();
    expect(screen.getByText("UI Cheats Extension")).toBeVisible();
    expect(screen.getByRole("button", { name: "Review update" })).toBeVisible();
  });
});
