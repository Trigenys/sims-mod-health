import type { GameContentHealthSnapshot } from "./gameContent.types";
import { gameContentMeasurement } from "./gameContent.presenter";
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
  const measurement = gameContentMeasurement(health, fallbackVersion);

  return (
    <section className="game-installation-summary" aria-label={t("Sims 4 installation summary")}>
      <SummaryFact
        label={t("Game build")}
        value={measurement.version ?? t("Not detected")}
        unavailable={!measurement.version}
      />
      <SummaryFact
        label={t("Installed packs")}
        value={measurement.packCount === null ? t("Not detected") : String(measurement.packCount)}
        unavailable={measurement.packCount === null}
      />
      <SummaryFact label={t("Mods & CC")} value={String(modCount)} />
      <SummaryFact
        label={t("Game / pack attention")}
        value={measurement.attentionCount === null ? t("Not checked") : String(measurement.attentionCount)}
        emphasized={(measurement.attentionCount ?? 0) > 0}
        unavailable={measurement.attentionCount === null}
      />
    </section>
  );
}

function SummaryFact({
  label,
  value,
  emphasized = false,
  unavailable = false
}: {
  label: string;
  value: string;
  emphasized?: boolean;
  unavailable?: boolean;
}) {
  const className = [
    "game-installation-summary__fact",
    emphasized ? "game-installation-summary__fact--attention" : "",
    unavailable ? "game-installation-summary__fact--unavailable" : ""
  ].filter(Boolean).join(" ");

  return (
    <div className={className}>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
