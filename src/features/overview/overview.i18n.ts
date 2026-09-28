import type { TranslationFn } from "../../i18n/i18n";

export function localizeOverviewExplanation(value: string, t: TranslationFn) {
  if (value === "Overall health is the percentage of current canonical releases with verified compatible patch evidence. Update-available releases still count as compatible when their installed release is compatible; unresolved files remain in the denominator.") {
    return t("Overall health is the percentage of current canonical releases with verified compatible patch evidence. Update-available releases still count as compatible when their installed release is compatible; unresolved files remain in the denominator.");
  }
  if (value === "Health becomes available after an installation has been scanned and resolved against the registry.") {
    return t("Health becomes available after an installation has been scanned and resolved against the registry.");
  }
  return value;
}

export function localizeOverviewDetail(value: string, t: TranslationFn) {
  const direct = directDetailKey(value);
  if (direct) return t(direct);

  let match = value.match(/^Artifact identity resolved, but health evaluation is incomplete: (.+)$/);
  if (match) {
    return t("Artifact identity resolved, but health evaluation is incomplete: {{error}}", { error: match[1] });
  }

  match = value.match(/^Compatibility is current, but dependency\/conflict data is partial: (.+)$/);
  if (match) {
    return t("Compatibility is current, but dependency/conflict data is partial: {{error}}", { error: match[1] });
  }

  match = value.match(/^Update available to (.+); installed compatibility is (.+)\.$/);
  if (match) {
    return t("Update available to {{target}}; installed compatibility is {{state}}.", {
      target: match[1],
      state: localizeCompatibility(match[2], t)
    });
  }

  match = value.match(/^Known incompatibility with (.+) is active for the installed versions\.$/);
  if (match) {
    return t("Known incompatibility with {{name}} is active for the installed versions.", { name: match[1] });
  }

  match = value.match(/^(\d+) exact copies share the same SHA-256 fingerprint\.$/);
  if (match) {
    return t("{{count}} exact copies share the same SHA-256 fingerprint.", { count: match[1] });
  }

  match = value.match(/^Potential conflict with (.+) across (\d+) shared DBPF resource keys\.$/);
  if (match) {
    return t("Potential conflict with {{name}} across {{count}} shared DBPF resource keys.", {
      name: match[1],
      count: match[2]
    });
  }

  return value;
}

export function localizeAttentionCreator(value: string, t: TranslationFn) {
  if (value === "Local scan") return t("Local scan");
  if (value === "Local library") return t("Local library");
  if (value === "Registry health") return t("Registry health");
  if (value === "Dependency graph") return t("Dependency graph");
  return value;
}

export function localizeAttentionBadge(value: string, t: TranslationFn) {
  const known = new Set([
    "Patch unknown",
    "Ambiguous",
    "Unknown",
    "Update",
    "Potential conflict",
    "Broken",
    "Abandoned",
    "Incompatible",
    "Missing dependency",
    "Dependency update",
    "Dependency review",
    "Duplicate",
    "Stale scan",
    "Partial"
  ]);
  return known.has(value) ? t(value as Parameters<TranslationFn>[0]) : value;
}

function directDetailKey(value: string): Parameters<TranslationFn>[0] | null {
  const values = [
    "Compatibility cannot be evaluated until the installed game patch is available.",
    "Local scan is available, but the installed game patch could not be read.",
    "No fingerprinted enabled artifacts are available for registry evaluation yet.",
    "Registry lookup completed, but no enabled artifact resolved to a canonical release.",
    "Registry health is available, but the current local scan contains partial observations.",
    "Local scan, artifact identity and registry health are current.",
    "Multiple registry candidates remain plausible; no identity was selected.",
    "No canonical registry identity could be established from the current evidence.",
    "Current-patch evidence reports a potential conflict. Review the evidence before changing files.",
    "Current-patch evidence marks this installed release as broken.",
    "The installed release is marked abandoned by current sourced evidence.",
    "Current evidence sources disagree, so compatibility remains Unknown.",
    "No current-patch compatibility evidence is available.",
    "A required dependency is not installed.",
    "An installed dependency is below the required version.",
    "An installed dependency does not satisfy the declared version range.",
    "The latest completed scan is older than 24 hours. Run an incremental scan before relying on the current picture.",
    "The previous scan did not complete, so the Overview may not include every installed file."
  ] as const;

  return values.includes(value as (typeof values)[number])
    ? (value as (typeof values)[number])
    : null;
}

function localizeCompatibility(value: string, t: TranslationFn) {
  if (value === "compatible") return t("compatible");
  if (value === "unknown") return t("unknown");
  if (value === "potential_conflict") return t("potential conflict");
  return value;
}
