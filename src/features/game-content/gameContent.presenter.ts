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
  if (state === "metadata_stale") return "Info may be outdated";
  if (state === "local_integrity_uncertain") return "Files need checking";
  return "Not enough information";
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
  return "Your game launcher";
}


export function localizedGameContentReason(
  finding: GameContentHealthFinding,
  t: TranslationFn
) {
  const current = finding.currentVersion ?? t("Unknown");
  const required = finding.requiredVersion ?? t("Unknown");

  if (finding.kind === "game") {
    if (finding.disputed) {
      return t("We found conflicting version information, so we will not guess whether you need an update.");
    }
    if (finding.state === "update_available") {
      return finding.manifestStale
        ? t("Cached metadata indicates game build {{required}} is newer than installed {{current}}.", { required, current })
        : t("Game build {{required}} is newer than installed {{current}}.", { required, current });
    }
    if (finding.state === "metadata_stale") {
      if (finding.currentVersion && finding.requiredVersion && finding.currentVersion !== finding.requiredVersion) {
        return t("Your installed version {{current}} is newer than the version we know about ({{required}}). Our online information needs refreshing.", { current, required });
      }
      return t("Your game looks up to date, but our online version information may be old.");
    }
    if (finding.state === "current") {
      return t("Your game matches the latest version we know about.");
    }
    if (!finding.currentVersion) {
      return t("We could not read your installed game version.");
    }
    return t("We do not have enough reliable information to compare game versions.");
  }

  if (finding.state === "local_integrity_uncertain") {
    return t("Some files for this pack are missing or could not be checked.");
  }
  if (finding.disputed) {
    return t("Our compatibility sources disagree about this pack, so we will not guess.");
  }
  if (finding.state === "game_update_required") {
    return t("{{code}} needs game version {{required}} or newer. You currently have {{current}}.", {
      code: finding.targetId,
      required,
      current
    });
  }
  if (finding.state === "metadata_stale") {
    return t("We do not see a compatibility problem, but our online information may be out of date.");
  }
  if (finding.state === "current") {
    return t("Your current game version meets this pack's known requirement.");
  }
  if (finding.requiredVersion && !finding.currentVersion) {
    return t("This pack needs a minimum game version, but we could not read your installed game version.");
  }
  if (!finding.requiredVersion) {
    return t("The pack is installed, but we do not have enough compatibility information for it yet.");
  }
  return t("We do not have enough reliable information to compare this pack with your game version.");
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
  return t("We could not identify your game launcher. Update The Sims 4 in the app you normally use, then come back and verify.");
}
