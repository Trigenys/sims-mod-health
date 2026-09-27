import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LibraryPage } from "./LibraryPage";
import { libraryVisualGateway } from "./library.gateway";

describe("LibraryPage", () => {
  it("searches canonical names and filename aliases, including unresolved files", async () => {
    render(<LibraryPage onOpenItem={vi.fn()} gateway={libraryVisualGateway} />);

    await screen.findByRole("button", { name: "Open MC Command Center" });
    const search = screen.getByLabelText("Search local filename");
    fireEvent.change(search, { target: { value: "CAS_Lighting_Golden.package" } });

    expect(screen.getByRole("button", { name: "Open CAS Lighting Override" })).toBeVisible();
    expect(screen.getByText("Unresolved")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Open MC Command Center" })).not.toBeInTheDocument();
  });

  it("filters identified and unknown inventory without hiding local usability", async () => {
    render(<LibraryPage onOpenItem={vi.fn()} gateway={libraryVisualGateway} />);

    await screen.findByRole("button", { name: "Open MC Command Center" });
    fireEvent.change(screen.getByLabelText("Filter by identification"), {
      target: { value: "Unknown" }
    });

    expect(screen.getByRole("button", { name: "Open CAS Lighting Override" })).toBeVisible();
    expect(screen.getByText("Local file · CAS_Lighting_Golden.package")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Open Better BuildBuy" })).not.toBeInTheDocument();
  });

  it("shows an actionable empty state and can clear filters", async () => {
    render(<LibraryPage onOpenItem={vi.fn()} gateway={libraryVisualGateway} />);

    await screen.findByRole("button", { name: "Open MC Command Center" });
    fireEvent.change(screen.getByLabelText("Search local filename"), {
      target: { value: "definitely-not-installed" }
    });

    expect(screen.getByRole("heading", { name: "No matching items" })).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
    expect(screen.getByRole("button", { name: "Open MC Command Center" })).toBeVisible();
  });

  it.each([
    ["offline", "Registry offline"],
    ["partial", "Partial registry results"],
    ["failure", "Registry request failed"]
  ] as const)("keeps the local library visible in %s state", async (state, message) => {
    render(<LibraryPage onOpenItem={vi.fn()} registryState={state} gateway={libraryVisualGateway} />);

    expect(await screen.findByText(message)).toBeVisible();
    expect(screen.getByRole("button", { name: "Open MC Command Center" })).toBeVisible();
  });

  it("keeps library rows keyboard focusable", async () => {
    render(<LibraryPage onOpenItem={vi.fn()} gateway={libraryVisualGateway} />);

    const row = await screen.findByRole("button", { name: "Open MC Command Center" });
    row.focus();

    expect(row).toHaveFocus();
  });
});
