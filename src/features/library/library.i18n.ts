import type { TranslationFn } from "../../i18n/i18n";

export function localizeLibraryGeneratedCopy(value: string, t: TranslationFn) {
  if (value === "This is a real file from the scanned Mods folder. Sims Mod Health has not resolved a canonical mod identity for it yet.") {
    return t("This is a real file from the scanned Mods folder. Sims Mod Health has not resolved a canonical mod identity for it yet.");
  }

  let match = value.match(
    /^The desktop scanner indexed this file from your selected Mods folder\. Parse state: (.+)\.$/
  );
  if (match) {
    return t("The desktop scanner indexed this file from your selected Mods folder. Parse state: {{state}}.", {
      state: localizeParseState(match[1], t)
    });
  }

  return value;
}

function localizeParseState(value: string, t: TranslationFn) {
  if (value === "parsed") return t("Parsed");
  if (value === "malformed") return t("Malformed");
  if (value === "unsupported") return t("Unsupported");
  if (value === "error") return t("Read error");
  return value;
}
