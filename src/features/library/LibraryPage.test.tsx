import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LibraryPage } from "./LibraryPage";

describe("LibraryPage", () => {
  it("searches canonical names and filename aliases, including unresolved files", () => {
    render(<LibraryPage onOpenItem={vi.fn()} />);

    const search = screen.getByLabelText("Search canonical name or filename");
    fireEvent.change(search, { target: { value: "CAS_Lighting_Golden.package" } });

    expect(screen.getByRole("button", { name: "Open CAS Lighting Override" })).toBeVisible();
    expect(screen.getByText("Unresolved")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Open MC Command Center" })).not.toBeInTheDocument();
  });

  it("filters identified and unknown inventory without hiding local usability", () => {
    render(<LibraryPage onOpenItem={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Filter by identification"), {
      target: { value: "Unknown" }
    });

    expect(screen.getByRole("button", { name: "Open CAS Lighting Override" })).toBeVisible();
    expect(screen.getByText("Local file · CAS_Lighting_Golden.package")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Open Better BuildBuy" })).not.toBeInTheDocument();
  });

  it("shows an actionable empty state and can clear filters", () => {
    render(<LibraryPage onOpenItem={vi.fn()} />);

    fireEvent.change(screen.getByLabelText("Search canonical name or filename"), {
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
  ] as const)("keeps the local library visible in %s state", (state, message) => {
    render(<LibraryPage onOpenItem={vi.fn()} registryState={state} />);

    expect(screen.getByText(message)).toBeVisible();
    expect(screen.getByRole("button", { name: "Open MC Command Center" })).toBeVisible();
  });

  it("keeps library rows keyboard focusable", () => {
    render(<LibraryPage onOpenItem={vi.fn()} />);

    const row = screen.getByRole("button", { name: "Open MC Command Center" });
    row.focus();

    expect(row).toHaveFocus();
  });
});
