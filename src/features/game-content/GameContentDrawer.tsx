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
            {finding.kind === "game" ? "GAME DETAIL" : "PACK DETAIL"}
          </span>
          <h2 id="game-content-detail-title">
            {finding.kind === "game" ? "The Sims 4" : finding.targetId}
          </h2>
        </div>
        <button
          ref={closeRef}
          type="button"
          className="game-content-drawer__close"
          aria-label="Close game content details"
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
          <dt>Installed build</dt>
          <dd>{finding.currentVersion ?? "Unknown"}</dd>
        </div>
        <div>
          <dt>{finding.kind === "game" ? "Latest known build" : "Minimum game build"}</dt>
          <dd>{finding.requiredVersion ?? "Not declared"}</dd>
        </div>
        <div>
          <dt>Evidence</dt>
          <dd>{finding.disputed ? "Disputed" : finding.manifestStale ? "Cached / stale" : "Current"}</dd>
        </div>
      </dl>

      <section className="game-content-evidence" aria-label="Compatibility evidence">
        <span className="section-kicker">WHY THIS STATE</span>
        {finding.evidence.length === 0 ? (
          <p>No trusted compatibility evidence is currently attached.</p>
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
          <span className="section-kicker">OFFICIAL UPDATE PROVIDER</span>
          <strong>{formatProvider(capability?.provider ?? "unknown")}</strong>
          <p>{capability?.detail ?? "Provider capability is being resolved."}</p>

          {sessionWaiting ? (
            <Button onClick={onVerify} disabled={busy}>
              {busy ? "Verifying…" : "I updated it — verify now"}
            </Button>
          ) : (
            <Button
              onClick={onUpdate}
              disabled={busy || capability === null || !capability.supported}
            >
              {busy ? "Opening…" : capability?.actionLabel ?? "Resolve provider"}
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
