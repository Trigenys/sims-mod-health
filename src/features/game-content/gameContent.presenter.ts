import type { StatusTone } from "../../components/ui/StatusBadge";
import type { TranslationFn } from "../../i18n/i18n";
import type {
  GameContentHealthFinding,
  GameContentHealthSnapshot,
  GameContentHealthState
} from "./gameContent.types";

export function gameContentTone(state: GameContentHealthState): StatusTone {
  if (state === "current") return "healthy";
  if (state === "update_available" || state === "game_update_required") return "update";
  if (state === "local_integrity_uncertain") return "warning";
  return "muted";
}

export function gameContentBadge(state: GameContentHealthState) {
  if (state === "current") return "Current";
  if (state === "update_available") return "Update";
  if (state === "game_update_required") return "Game update required";
  if (state === "metadata_stale") return "Metadata stale";
  if (state === "local_integrity_uncertain") return "Integrity uncertain";
  return "Unknown";
}

export function isActionableGameContent(finding: GameContentHealthFinding) {
  return finding.state !== "current";
}

export function isUpdateGameContent(finding: GameContentHealthFinding) {
  return finding.state === "update_available" || finding.state === "game_update_required";
}

export function packHealthSummary(snapshot: GameContentHealthSnapshot | null) {
  const packs = snapshot?.packs ?? [];
  return {
    total: packs.length,
    current: packs.filter((finding) => finding.state === "current").length,
    attention: packs.filter(isActionableGameContent).length
  };
}

export function gameContentAttentionCount(snapshot: GameContentHealthSnapshot | null) {
  if (!snapshot) return 0;
  return [snapshot.game, ...snapshot.packs]
    .filter((finding): finding is GameContentHealthFinding => finding !== null)
    .filter(isActionableGameContent).length;
}

export function formatProvider(provider: string) {
  if (provider === "ea_app") return "EA app";
  if (provider === "steam") return "Steam";
  return "Manual provider";
}


export function localizedGameContentReason(
  finding: GameContentHealthFinding,
  t: TranslationFn
) {
  const current = finding.currentVersion ?? t("Unknown");
  const required = finding.requiredVersion ?? t("Unknown");

  if (finding.kind === "game") {
    if (finding.disputed) {
      return t("Game-build evidence conflicts, so no update conclusion is safe.");
    }
    if (finding.state === "update_available") {
      return finding.manifestStale
        ? t("Cached metadata indicates game build {{required}} is newer than installed {{current}}.", { required, current })
        : t("Game build {{required}} is newer than installed {{current}}.", { required, current });
    }
    if (finding.state === "metadata_stale") {
      if (finding.currentVersion && finding.requiredVersion && finding.currentVersion !== finding.requiredVersion) {
        return t("Installed build {{current}} is newer than manifest latest {{required}}; metadata needs refresh.", { current, required });
      }
      return t("Installed game matches the cached latest build, but Registry metadata is stale.");
    }
    if (finding.state === "current") {
      return t("Installed game matches the latest known build.");
    }
    if (!finding.currentVersion) {
      return t("Installed game build could not be resolved from trusted local evidence.");
    }
    return t("Game versions could not be compared safely.");
  }

  if (finding.state === "local_integrity_uncertain") {
    return t("Pack files are incomplete or could not be inspected reliably.");
  }
  if (finding.disputed) {
    return t("Trusted metadata sources disagree about this pack's compatibility requirements.");
  }
  if (finding.state === "game_update_required") {
    return t("{{code}} requires game build {{required}} or newer; installed build is {{current}}.", {
      code: finding.targetId,
      required,
      current
    });
  }
  if (finding.state === "metadata_stale") {
    return t("Cached metadata does not show a compatibility problem, but it is stale.");
  }
  if (finding.state === "current") {
    return t("Installed game satisfies the pack's known compatibility requirement.");
  }
  if (finding.requiredVersion && !finding.currentVersion) {
    return t("Pack has a minimum game build, but the installed game build is unknown.");
  }
  if (!finding.requiredVersion) {
    return t("Pack is installed, but the manifest has no compatibility metadata for it.");
  }
  return t("Pack minimum version could not be compared safely.");
}

export function localizedEvidenceDetail(
  detail: string,
  t: TranslationFn
) {
  let match = detail.match(/^Installed game version (.+) was read locally\.$/);
  if (match) {
    return t("Installed game version {{current}} was read locally.", { current: match[1] });
  }

  match = detail.match(/^Sentinel fingerprints resolve uniquely to game build (.+)\.$/);
  if (match) {
    return t("Sentinel fingerprints resolve uniquely to game build {{current}}.", { current: match[1] });
  }

  match = detail.match(/^Manifest latest game build is (.+)\.$/);
  if (match) {
    return t("Manifest latest game build is {{required}}.", { required: match[1] });
  }

  match = detail.match(/^Minimum game build: (.+)\.$/);
  if (match) {
    return t("Minimum game build: {{required}}.", { required: match[1] });
  }

  if (detail === "No minimum game build is declared.") {
    return t("No minimum game build is declared.");
  }

  return detail;
}

export function providerActionLabel(provider: string, t: TranslationFn) {
  if (provider === "ea_app") return t("Open EA app to update");
  if (provider === "steam") return t("Open Steam to update");
  return t("Update in your game provider");
}

export function providerCapabilityDetail(
  provider: string,
  supported: boolean,
  t: TranslationFn
) {
  if (provider === "ea_app") {
    return supported
      ? t("EA app is available. Sims Mod Health will open the official client and wait for local verification.")
      : t("EA app was detected for the game, but EADesktop.exe could not be located. Open EA app manually, update The Sims 4, then return to verify.");
  }
  if (provider === "steam") {
    return supported
      ? t("Steam is available. Sims Mod Health will open the official client and wait for local verification.")
      : t("Steam was detected for the game, but steam.exe could not be located. Open Steam manually, update The Sims 4, then return to verify.");
  }
  return t("The update provider could not be identified. Update The Sims 4 in the client you normally use, then return to verify.");
}
