import { useEffect, useRef } from "react";
import { Button } from "../../components/ui/Button";
import { StatusBadge } from "../../components/ui/StatusBadge";
import {
  formatProvider,
  gameContentBadge,
  gameContentTone,
  isUpdateGameContent,
  localizedEvidenceDetail,
  localizedGameContentReason,
  providerActionLabel,
  providerCapabilityDetail
} from "./gameContent.presenter";
import type {
  GameContentHealthFinding,
  ProviderUpdateCapability,
  ProviderUpdateSession
} from "./gameContent.types";
import { useI18n } from "../../i18n/i18n";

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
  const { t, tx } = useI18n();
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
            {finding.kind === "game" ? t("GAME STATUS") : t("PACK STATUS")}
          </span>
          <h2 id="game-content-detail-title">
            {finding.kind === "game" ? "The Sims 4" : finding.targetId}
          </h2>
        </div>
        <button
          ref={closeRef}
          type="button"
          className="game-content-drawer__close"
          aria-label={t("Close game content details")}
          onClick={onClose}
        >
          ×
        </button>
      </div>

      <StatusBadge tone={gameContentTone(finding.state)}>
        {tx(gameContentBadge(finding.state))}
      </StatusBadge>

      <p className="game-content-drawer__reason">{localizedGameContentReason(finding, t)}</p>

      <dl className="game-content-drawer__facts">
        <div>
          <dt>{t("Installed build")}</dt>
          <dd>{finding.currentVersion ?? t("Unknown")}</dd>
        </div>
        <div>
          <dt>{finding.kind === "game" ? t("Latest known build") : t("Minimum game build")}</dt>
          <dd>{finding.requiredVersion ?? t("Not declared")}</dd>
        </div>
        <div>
          <dt>{t("Check status")}</dt>
          <dd>{finding.disputed ? t("Sources disagree") : finding.manifestStale ? t("Info may be outdated") : t("Current")}</dd>
        </div>
      </dl>

      <section className="game-content-evidence" aria-label={t("Compatibility evidence")}>
        <span className="section-kicker">{t("WHY YOU ARE SEEING THIS")}</span>
        {finding.evidence.length === 0 ? (
          <p>{t("We do not have enough reliable compatibility information for this yet.")}</p>
        ) : (
          <ul>
            {finding.evidence.map((item, index) => (
              <li key={item.source + "-" + index}>
                <strong>{item.source}</strong>
                <span>{localizedEvidenceDetail(item.detail, t)}</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      {updateEligible && (
        <section className="game-content-provider-action">
          <span className="section-kicker">{t("UPDATE WITH")}</span>
          <strong>{formatProvider(capability?.provider ?? "unknown")}</strong>
          <p>{capability ? providerCapabilityDetail(capability.provider, capability.supported, t) : t("Checking which app should handle the update…")}</p>

          {sessionWaiting ? (
            <Button onClick={onVerify} disabled={busy}>
              {busy ? t("Verifying…") : t("I updated it — verify now")}
            </Button>
          ) : (
            <Button
              onClick={onUpdate}
              disabled={busy || capability === null || !capability.supported}
            >
              {busy ? t("Opening…") : capability ? providerActionLabel(capability.provider, t) : t("Find update app")}
            </Button>
          )}

          {session && session.targetId === finding.targetId && (
            <small role="status">{tx(session.detail)}</small>
          )}
          {error && <small className="game-content-provider-action__error">{error}</small>}
        </section>
      )}
    </aside>
  );
}
