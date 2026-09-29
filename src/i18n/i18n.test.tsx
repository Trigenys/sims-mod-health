import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { I18nProvider, resolveLocale, useI18n } from "./i18n";

function Probe() {
  const { locale, setLocale, t } = useI18n();
  return (
    <div>
      <span data-testid="locale">{locale}</span>
      <strong>{t("Settings")}</strong>
      <button type="button" onClick={() => setLocale("fr")}>fr</button>
      <button type="button" onClick={() => setLocale("en")}>en</button>
    </div>
  );
}

describe("desktop i18n", () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.lang = "";
  });

  it("prefers a persisted locale over the system locale", () => {
    expect(resolveLocale({ stored: "en", system: "fr-FR" })).toBe("en");
    expect(resolveLocale({ stored: "fr", system: "en-US" })).toBe("fr");
  });

  it("falls back to French for a French system locale and English otherwise", () => {
    expect(resolveLocale({ stored: null, system: "fr-CM" })).toBe("fr");
    expect(resolveLocale({ stored: null, system: "en-US" })).toBe("en");
    expect(resolveLocale({ stored: null, system: null })).toBe("en");
  });

  it("switches immediately and persists the selected language", () => {
    window.localStorage.setItem("sims-mod-health.locale", "en");

    render(
      <I18nProvider>
        <Probe />
      </I18nProvider>
    );

    expect(screen.getByText("Settings")).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "fr" }));

    expect(screen.getByText("Paramètres")).toBeVisible();
    expect(screen.getByTestId("locale")).toHaveTextContent("fr");
    expect(window.localStorage.getItem("sims-mod-health.locale")).toBe("fr");
    expect(document.documentElement.lang).toBe("fr");
  });
});
