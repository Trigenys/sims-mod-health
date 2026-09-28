import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { I18nProvider, STORAGE_KEY, useI18n } from "./I18nProvider";

function Probe() {
  const { locale, setLocale, t } = useI18n();
  return (
    <div>
      <span data-testid="locale">{locale}</span>
      <span>{t("nav.overview")}</span>
      <button onClick={() => setLocale("fr")}>fr</button>
      <button onClick={() => setLocale("en")}>en</button>
    </div>
  );
}

describe("I18nProvider", () => {
  afterEach(() => {
    window.localStorage.removeItem(STORAGE_KEY);
  });

  it("persists a runtime locale change", () => {
    render(
      <I18nProvider>
        <Probe />
      </I18nProvider>
    );

    fireEvent.click(screen.getByRole("button", { name: "fr" }));
    expect(screen.getByTestId("locale")).toHaveTextContent("fr");
    expect(screen.getByText("Vue d’ensemble")).toBeVisible();
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe("fr");
  });

  it("uses the stored locale on startup", () => {
    window.localStorage.setItem(STORAGE_KEY, "fr");

    render(
      <I18nProvider>
        <Probe />
      </I18nProvider>
    );

    expect(screen.getByTestId("locale")).toHaveTextContent("fr");
    expect(screen.getByText("Vue d’ensemble")).toBeVisible();
  });
});
