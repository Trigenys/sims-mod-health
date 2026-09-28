import { useEffect, useRef } from "react";
import { Button } from "../../components/ui/Button";
import { StatusBadge } from "../../components/ui/StatusBadge";
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
            {finding.kind === "game" ? t("GAME DETAIL") : t("PACK DETAIL")}
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

      <p className="game-content-drawer__reason">{tx(finding.reason)}</p>

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
          <dt>{t("Evidence")}</dt>
          <dd>{finding.disputed ? t("Disputed") : finding.manifestStale ? t("Cached / stale") : t("Current")}</dd>
        </div>
      </dl>

      <section className="game-content-evidence" aria-label={t("Compatibility evidence")}>
        <span className="section-kicker">{t("WHY THIS STATE")}</span>
        {finding.evidence.length === 0 ? (
          <p>{t("No trusted compatibility evidence is currently attached.")}</p>
        ) : (
          <ul>
            {finding.evidence.map((item, index) => (
              <li key={item.source + "-" + index}>
                <strong>{item.source}</strong>
                <span>{tx(item.detail)}</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      {updateEligible && (
        <section className="game-content-provider-action">
          <span className="section-kicker">{t("OFFICIAL UPDATE PROVIDER")}</span>
          <strong>{formatProvider(capability?.provider ?? "unknown")}</strong>
          <p>{capability?.detail ? tx(capability.detail) : t("Provider capability is being resolved.")}</p>

          {sessionWaiting ? (
            <Button onClick={onVerify} disabled={busy}>
              {busy ? t("Verifying…") : t("I updated it — verify now")}
            </Button>
          ) : (
            <Button
              onClick={onUpdate}
              disabled={busy || capability === null || !capability.supported}
            >
              {busy ? t("Opening…") : capability?.actionLabel ? tx(capability.actionLabel) : t("Resolve provider")}
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
