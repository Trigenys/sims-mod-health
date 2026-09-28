import type { StatusTone } from "../../components/ui/StatusBadge";
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
