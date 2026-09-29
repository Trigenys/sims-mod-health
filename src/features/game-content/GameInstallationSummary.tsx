import type { GameContentHealthSnapshot } from "./gameContent.types";
import { gameContentAttentionCount, packHealthSummary } from "./gameContent.presenter";
import { useI18n } from "../../i18n/i18n";

export function GameInstallationSummary({
  health,
  modCount,
  fallbackVersion
}: {
  health: GameContentHealthSnapshot | null;
  modCount: number;
  fallbackVersion: string | null;
}) {
  const { t } = useI18n();
  const packs = packHealthSummary(health);
  const attention = gameContentAttentionCount(health);
  const version = health?.game?.currentVersion ?? fallbackVersion;

  return (
    <section className="game-installation-summary" aria-label={t("Sims 4 installation summary")}>
      <SummaryFact label={t("Game build")} value={version ?? t("Unknown")} />
      <SummaryFact label={t("Installed packs")} value={String(packs.total)} />
      <SummaryFact label={t("Mods & CC")} value={String(modCount)} />
      <SummaryFact
        label={t("Game / pack attention")}
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
