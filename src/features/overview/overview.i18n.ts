import type { TranslationFn } from "../../i18n/i18n";

export function localizeOverviewExplanation(value: string, t: TranslationFn) {
  if (value === "Overall health is the percentage of current canonical releases with verified compatible patch evidence. Update-available releases still count as compatible when their installed release is compatible; unresolved files remain in the denominator.") {
    return t("This score uses the mods we could identify and check against your current game version. Files we cannot identify yet lower the score instead of being guessed.");
  }
  if (value === "Health becomes available after an installation has been scanned and resolved against the registry.") {
    return t("Run a complete scan first. We need your game version and Mods folder before we can rate the setup.");
  }
  return value;
}

export function localizeOverviewDetail(value: string, t: TranslationFn) {
  const direct = directDetailKey(value);
  if (direct) return t(direct);

  let match = value.match(/^Artifact identity resolved, but health evaluation is incomplete: (.+)$/);
  if (match) {
    return t("We identified the mod, but could not finish checking its compatibility. Try again later.");
  }

  match = value.match(/^Compatibility is current, but dependency\/conflict data is partial: (.+)$/);
  if (match) {
    return t("This mod looks compatible, but some dependency or conflict checks could not be completed.");
  }

  match = value.match(/^Update available to (.+); installed compatibility is (.+)\.$/);
  if (match) {
    return t("A newer version ({{target}}) is available. Your installed version currently looks {{state}}.", {
      target: match[1],
      state: localizeCompatibility(match[2], t)
    });
  }

  match = value.match(/^Known incompatibility with (.+) is active for the installed versions\.$/);
  if (match) {
    return t("This version is known to conflict with {{name}}.", { name: match[1] });
  }

  match = value.match(/^(\d+) exact copies share the same SHA-256 fingerprint\.$/);
  if (match) {
    return t("{{count}} identical copies of this file were found.", { count: match[1] });
  }

  match = value.match(/^Potential conflict with (.+) across (\d+) shared DBPF resource keys\.$/);
  if (match) {
    return t("This file may overlap with {{name}}. Review both mods before removing anything.", {
      name: match[1]
    });
  }

  return value;
}

export function localizeAttentionCreator(value: string, t: TranslationFn) {
  if (value === "Local scan") return t("Local scan");
  if (value === "Local library") return t("Local library");
  if (value === "Registry health") return t("Online compatibility check");
  if (value === "Dependency graph") return t("Required mods check");
  return value;
}

export function localizeAttentionBadge(value: string, t: TranslationFn) {
  if (value === "Patch unknown") return t("Game version not detected yet");
  if (value === "Ambiguous") return t("Several possible matches");
  if (value === "Unknown") return t("Not identified yet");
  if (value === "Stale scan") return t("Scan out of date");
  if (value === "Partial") return t("Some checks incomplete");

  const known = new Set([
    "Update",
    "Potential conflict",
    "Broken",
    "Abandoned",
    "Incompatible",
    "Missing dependency",
    "Dependency update",
    "Dependency review",
    "Duplicate"
  ]);
  return known.has(value) ? t(value as Parameters<TranslationFn>[0]) : value;
}

function directDetailKey(value: string): Parameters<TranslationFn>[0] | null {
  const copy: Record<string, Parameters<TranslationFn>[0]> = {
    "Compatibility cannot be evaluated until the installed game patch is available.":
      "We need your game version before we can check mod compatibility.",
    "Local scan is available, but the installed game patch could not be read.":
      "Your mods were scanned, but we could not read the game version. Check the game folder in Settings.",
    "No fingerprinted enabled artifacts are available for registry evaluation yet.":
      "We found no enabled mod files that can be checked online yet.",
    "Registry lookup completed, but no enabled artifact resolved to a canonical release.":
      "We found your files, but could not identify any enabled mods online yet.",
    "Registry health is available, but the current local scan contains partial observations.":
      "Online checks are available, but the last local scan was incomplete. Scan again for a fuller result.",
    "Local scan, artifact identity and registry health are current.":
      "Your local scan and online checks are up to date.",
    "Multiple registry candidates remain plausible; no identity was selected.":
      "This file matches several possible mods, so we did not choose one automatically.",
    "No canonical registry identity could be established from the current evidence.":
      "We found this file, but could not identify the exact mod yet.",
    "Current-patch evidence reports a potential conflict. Review the evidence before changing files.":
      "This mod may conflict with another mod on your current game version. Review the details before changing files.",
    "Current-patch evidence marks this installed release as broken.":
      "This installed mod version is reported as broken for your current game version.",
    "The installed release is marked abandoned by current sourced evidence.":
      "This mod version is no longer maintained according to the information we have.",
    "Current evidence sources disagree, so compatibility remains Unknown.":
      "Our compatibility sources disagree, so we marked this as not confirmed.",
    "No current-patch compatibility evidence is available.":
      "We do not have enough compatibility information for this mod on your current game version.",
    "A required dependency is not installed.":
      "This mod needs another mod that is not installed.",
    "An installed dependency is below the required version.":
      "One required mod is installed, but it needs an update.",
    "An installed dependency does not satisfy the declared version range.":
      "One required mod does not meet this mod's version requirement.",
    "The latest completed scan is older than 24 hours. Run an incremental scan before relying on the current picture.":
      "Your last scan is over 24 hours old. Scan again for current results.",
    "The previous scan did not complete, so the Overview may not include every installed file.":
      "The last scan did not finish, so some installed files may be missing from these results.",
    "Scan stopped before finishing": "Scan stopped before finishing",
    "Try again. If it keeps failing, check that your Sims 4 folders are still available.":
      "Try again. If it keeps failing, check that your Sims 4 folders are still available.",
    "Scanning your mods": "Scanning your mods",
    "We are checking your local files now. Compatibility and update checks refresh when the scan finishes.":
      "We are checking your local files now. Compatibility and update checks refresh when the scan finishes.",
    "Your scan is out of date": "Your scan is out of date",
    "Run Scan again to refresh the results before relying on them.":
      "Run Scan again to refresh the results before relying on them.",
    "Some files could not be checked": "Some files could not be checked",
    "Your current results are still available, but scanning again may fill in the missing details.":
      "Your current results are still available, but scanning again may fill in the missing details.",
    "Online checks are temporarily unavailable": "Online checks are temporarily unavailable",
    "Your local scan still works. Mod names, compatibility and update information may be incomplete until online checks are available again.":
      "Your local scan still works. Mod names, compatibility and update information may be incomplete until online checks are available again.",
    "Some online checks are unavailable": "Some online checks are unavailable",
    "Your local scan is available, but some compatibility and update details could not be loaded.":
      "Your local scan is available, but some compatibility and update details could not be loaded."
  };

  return copy[value] ?? null;
}

function localizeCompatibility(value: string, t: TranslationFn) {
  if (value === "compatible") return t("compatible");
  if (value === "unknown") return t("unknown");
  if (value === "potential_conflict") return t("potential conflict");
  return value;
}
