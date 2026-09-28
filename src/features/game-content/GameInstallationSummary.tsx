import type { GameContentHealthSnapshot } from "./gameContent.types";
import { gameContentAttentionCount, packHealthSummary } from "./gameContent.presenter";

export function GameInstallationSummary({
  health,
  modCount,
  fallbackVersion
}: {
  health: GameContentHealthSnapshot | null;
  modCount: number;
  fallbackVersion: string | null;
}) {
  const packs = packHealthSummary(health);
  const attention = gameContentAttentionCount(health);
  const version = health?.game?.currentVersion ?? fallbackVersion;

  return (
    <section className="game-installation-summary" aria-label="Sims 4 installation summary">
      <SummaryFact label="Game build" value={version ?? "Unknown"} />
      <SummaryFact label="Installed packs" value={String(packs.total)} />
      <SummaryFact label="Mods & CC" value={String(modCount)} />
      <SummaryFact
        label="Game / pack attention"
        value={String(attention)}
        emphasized={attention > 0}
      />
    </section>
  );
}

function SummaryFact({
  label,
  value,
  emphasized = false
}: {
  label: string;
  value: string;
  emphasized?: boolean;
}) {
  return (
    <div className={emphasized ? "game-installation-summary__fact game-installation-summary__fact--attention" : "game-installation-summary__fact"}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
