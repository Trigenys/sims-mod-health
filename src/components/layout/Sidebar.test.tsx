import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Sidebar } from "./Sidebar";

describe("Sidebar", () => {
  it("exposes the active destination and accessible navigation names", () => {
    render(<Sidebar activeItem="Overview" />);

    expect(screen.getByRole("navigation", { name: "Primary navigation" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Overview" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("button", { name: "Library" })).not.toHaveAttribute("aria-current");
    expect(screen.getByLabelText("21 available updates")).toHaveTextContent("21");
  });
});
