import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Sidebar } from "./Sidebar";

describe("Sidebar", () => {
  it("exposes only the approved primary destinations", () => {
    render(<Sidebar activeItem="Overview" healthCount={6} />);

    expect(
      screen.getByRole("navigation", { name: "Primary navigation" })
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Overview" })).toHaveAttribute(
      "aria-current",
      "page"
    );
    expect(screen.getByRole("button", { name: "Library" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Health" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Discover" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Settings" })).toBeInTheDocument();
    expect(screen.getByLabelText("6 health findings")).toHaveTextContent("6");

    expect(screen.queryByRole("button", { name: "Updates" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Conflicts" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Diagnostics" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Backups" })).not.toBeInTheDocument();
  });

  it("emits the selected primary and utility destinations", () => {
    const onNavigate = vi.fn();
    render(<Sidebar activeItem="Overview" onNavigate={onNavigate} />);

    fireEvent.click(screen.getByRole("button", { name: "Health" }));
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(onNavigate).toHaveBeenNthCalledWith(1, "Health");
    expect(onNavigate).toHaveBeenNthCalledWith(2, "Settings");
  });
});
