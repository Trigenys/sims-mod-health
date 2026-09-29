import type { TranslationFn } from "../../i18n/i18n";

export function localizeLibraryGeneratedCopy(value: string, t: TranslationFn) {
  if (value === "This is a real file from the scanned Mods folder. Sims Mod Health has not resolved a canonical mod identity for it yet.") {
    return t("We found this file in your Mods folder, but we cannot identify the exact mod yet. We will not guess.");
  }

  let match = value.match(
    /^The desktop scanner indexed this file from your selected Mods folder\. Parse state: (.+)\.$/
  );
  if (match) {
    return t("This file was found in your Mods folder. File check: {{state}}.", {
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
