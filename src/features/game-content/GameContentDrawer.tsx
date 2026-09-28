import { useEffect, useRef } from "react";
import { Button } from "../../components/ui/Button";
import { StatusBadge } from "../../components/ui/StatusBadge";
import { useI18n } from "../../i18n/I18nProvider";
import {
  formatProvider,
  gameContentBadge,
  gameContentTone,
  isUpdateGameContent
} from "./gameContent.presenter";
import type {
  GameContentHealthFinding,
  ProviderUpdateCapability,
  ProviderUpdateSession
} from "./gameContent.types";

export function GameContentDrawer({
  finding,
  capability,
  session,
  busy,
  error,
  onClose,
  onUpdate,
  onVerify
}: {
  finding: GameContentHealthFinding;
  capability: ProviderUpdateCapability | null;
  session: ProviderUpdateSession | null;
  busy: boolean;
  error: string | null;
  onClose: () => void;
  onUpdate: () => void;
  onVerify: () => void;
}) {
  const { t } = useI18n();
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    closeRef.current?.focus();
  }, [finding.targetId]);

  const updateEligible = isUpdateGameContent(finding);
  const sessionWaiting =
    session?.targetId === finding.targetId && session.state === "awaiting_rescan";

  return (
    <aside
      className="game-content-drawer"
      role="dialog"
      aria-modal="false"
      aria-labelledby="game-content-detail-title"
    >
      <div className="game-content-drawer__header">
        <div>
          <span className="section-kicker">
            {finding.kind === "game" ? t("drawer.gameDetail") : t("drawer.packDetail")}
          </span>
          <h2 id="game-content-detail-title">
            {finding.kind === "game" ? "The Sims 4" : finding.targetId}
          </h2>
        </div>
        <button
          ref={closeRef}
          type="button"
          className="game-content-drawer__close"
          aria-label={t("drawer.close")}
          onClick={onClose}
        >
          ×
        </button>
      </div>

      <StatusBadge tone={gameContentTone(finding.state)}>
        {gameContentBadge(finding.state)}
      </StatusBadge>

      <p className="game-content-drawer__reason">{finding.reason}</p>

      <dl className="game-content-drawer__facts">
        <div>
          <dt>{t("drawer.installedBuild")}</dt>
          <dd>{finding.currentVersion ?? t("common.unknown")}</dd>
        </div>
        <div>
          <dt>{finding.kind === "game" ? t("drawer.latestBuild") : t("drawer.minimumBuild")}</dt>
          <dd>{finding.requiredVersion ?? t("drawer.notDeclared")}</dd>
        </div>
        <div>
          <dt>{t("drawer.evidence")}</dt>
          <dd>{finding.disputed ? t("drawer.disputed") : finding.manifestStale ? t("drawer.cachedStale") : t("common.current")}</dd>
        </div>
      </dl>

      <section className="game-content-evidence" aria-label={t("drawer.evidence")}>
        <span className="section-kicker">{t("drawer.why")}</span>
        {finding.evidence.length === 0 ? (
          <p>{t("drawer.noEvidence")}</p>
        ) : (
          <ul>
            {finding.evidence.map((item, index) => (
              <li key={item.source + "-" + index}>
                <strong>{item.source}</strong>
                <span>{item.detail}</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      {updateEligible && (
        <section className="game-content-provider-action">
          <span className="section-kicker">{t("drawer.provider")}</span>
          <strong>{formatProvider(capability?.provider ?? "unknown")}</strong>
          <p>{capability?.detail ?? t("drawer.resolvingProvider")}</p>

          {sessionWaiting ? (
            <Button onClick={onVerify} disabled={busy}>
              {busy ? t("drawer.verifying") : t("drawer.verifyNow")}
            </Button>
          ) : (
            <Button
              onClick={onUpdate}
              disabled={busy || capability === null || !capability.supported}
            >
              {busy ? t("drawer.opening") : capability?.actionLabel ?? t("drawer.resolveProvider")}
            </Button>
          )}

          {session && session.targetId === finding.targetId && (
            <small role="status">{session.detail}</small>
          )}
          {error && <small className="game-content-provider-action__error">{error}</small>}
        </section>
      )}
    </aside>
  );
}
