import { useI18n } from "../../i18n/I18nProvider";
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
  const { t } = useI18n();
  const packs = packHealthSummary(health);
  const attention = gameContentAttentionCount(health);
  const version = health?.game?.currentVersion ?? fallbackVersion;

  return (
    <section className="game-installation-summary" aria-label="Sims 4 installation summary">
      <SummaryFact label={t("gameSummary.build")} value={version ?? "Unknown"} />
      <SummaryFact label={t("gameSummary.packs")} value={String(packs.total)} />
      <SummaryFact label={t("gameSummary.mods")} value={String(modCount)} />
      <SummaryFact
        label={t("gameSummary.attention")}
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
