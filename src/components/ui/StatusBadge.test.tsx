import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusBadge } from "./StatusBadge";

describe("StatusBadge", () => {
  it("always exposes a text status instead of relying on color", () => {
    render(<StatusBadge tone="warning">Potential conflict</StatusBadge>);

    expect(screen.getByText("Potential conflict")).toBeVisible();
    expect(screen.getByText("!")).toHaveAttribute("aria-hidden", "true");
  });
});
