import { useEffect, useMemo, useState } from "react";
import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge, type StatusTone } from "../../components/ui/StatusBadge";
import { DiagnosticsPage } from "../diagnostics/DiagnosticsPage";
import { overviewGateway, type OverviewGateway } from "../overview/overview.gateway";
import type {
  OverviewAttentionItem,
  OverviewSnapshot
} from "../overview/overview.types";

export type HealthTab =
  | "all"
  | "updates"
  | "conflicts"
  | "diagnostics"
  | "recovery";

type HealthPageProps = {
  gateway?: OverviewGateway;
  initialTab?: HealthTab;
};

const tabs: { value: HealthTab; label: string }[] = [
  { value: "all", label: "All findings" },
  { value: "updates", label: "Updates" },
  { value: "conflicts", label: "Conflicts" },
  { value: "diagnostics", label: "Diagnostics" },
  { value: "recovery", label: "Recovery" }
];

export function HealthPage({
  gateway = overviewGateway,
  initialTab = "all"
}: HealthPageProps) {
  const [snapshot, setSnapshot] = useState<OverviewSnapshot | null>(null);
  const [tab, setTab] = useState<HealthTab>(initialTab);

  useEffect(() => {
    let active = true;
    gateway.load().then((value) => {
      if (active) setSnapshot(value);
    });
    return () => {
      active = false;
    };
  }, [gateway]);

  const findings = useMemo(
    () => filterFindings(snapshot?.attention ?? [], tab),
    [snapshot, tab]
  );

  if (!snapshot) {
    return (
      <section className="health-loading" role="status">
        Loading current health evidence…
      </section>
    );
  }

  return (
    <>
      <Topbar
        gameVersion={snapshot.gameVersion ? "Patch " + snapshot.gameVersion : "Patch unknown"}
        platform={formatPlatform(snapshot.platform)}
        indexedCount={snapshot.indexedCount}
      />

      <section className="health-hero" aria-labelledby="health-title">
        <div>
          <p className="eyebrow">HEALTH & ACTION CENTER</p>
          <h1 id="health-title">Review what needs attention.</h1>
          <p className="lede">
            Updates, conflicts, diagnostics and recovery live in one evidence-first workspace.
          </p>
        </div>

        <div className="health-hero__actions">
          <div className="health-snapshot">
            <span>Current health</span>
            <strong>{snapshot.healthScore ?? "—"}{snapshot.healthScore !== null ? "%" : ""}</strong>
          </div>
          <Button variant="primary">Review safest actions</Button>
        </div>
      </section>

      <section className="health-metrics" aria-label="Health summary">
        <Metric label="Needs attention" value={snapshot.attentionCount} tone="danger" />
        <Metric label="Updates" value={snapshot.healthCounts.updates} tone="update" />
        <Metric label="Conflicts" value={snapshot.healthCounts.conflicts} tone="warning" />
        <Metric label="Unknown" value={snapshot.healthCounts.unknown} tone="muted" />
        <Metric label="Healthy" value={snapshot.healthCounts.healthy} tone="healthy" />
      </section>

      <nav className="health-tabs" aria-label="Health views">
        {tabs.map((item) => {
          const active = tab === item.value;
          return (
            <button
              key={item.value}
              type="button"
              aria-current={active ? "page" : undefined}
              className={active ? "health-tab health-tab--active" : "health-tab"}
              onClick={() => setTab(item.value)}
            >
              {item.label}
              {item.value === "updates" && snapshot.healthCounts.updates > 0 && (
                <span>{snapshot.healthCounts.updates}</span>
              )}
              {item.value === "conflicts" && snapshot.healthCounts.conflicts > 0 && (
                <span>{snapshot.healthCounts.conflicts}</span>
              )}
            </button>
          );
        })}
      </nav>

      {tab === "diagnostics" ? (
        <section className="health-diagnostics" aria-label="Diagnostic evidence">
          <DiagnosticsPage embedded />
        </section>
      ) : tab === "recovery" ? (
        <RecoveryPanel />
      ) : (
        <div className="health-layout">
          <section className="health-findings" aria-label={tabLabel(tab)}>
            <div className="health-section-heading">
              <div>
                <span className="section-kicker">{tabLabel(tab).toUpperCase()}</span>
                <h2>{healthHeading(tab)}</h2>
              </div>
              <span>{findings.length} shown from current evidence</span>
            </div>

            {findings.length === 0 ? (
              <Panel className="health-empty">
                <div className="health-empty__icon" aria-hidden="true">✓</div>
                <div>
                  <h3>{emptyTitle(tab)}</h3>
                  <p>{emptyCopy(tab, snapshot)}</p>
                </div>
              </Panel>
            ) : (
              <div className="health-finding-list">
                {findings.map((finding, index) => (
                  <FindingCard
                    finding={finding}
                    key={finding.name + "-" + finding.badge + "-" + index}
                  />
                ))}
              </div>
            )}
          </section>

          <aside className="health-guardrails">
            <Panel>
              <span className="section-kicker">RECOVERY GUARANTEES</span>
              <h2>Actions stay reversible.</h2>
              <p>
                Supported mutations create a verified restore point before the Mods folder changes.
              </p>
              <ul>
                <li><span>✓</span> Restore point before supported updates</li>
                <li><span>✓</span> SHA-256 integrity checks</li>
                <li><span>✓</span> Interrupted transactions remain recoverable</li>
                <li><span>✓</span> Rollback refuses unsafe overwrites</li>
              </ul>
              <button
                type="button"
                className="health-link"
                onClick={() => setTab("recovery")}
              >
                Open Recovery
                <span aria-hidden="true">→</span>
              </button>
            </Panel>

            <Panel className="health-safety">
              <span className="section-kicker">EVIDENCE POLICY</span>
              <h2>Unknown is not broken.</h2>
              <p>
                The app keeps correlation, potential conflicts and deterministic failures visually distinct.
              </p>
            </Panel>
          </aside>
        </div>
      )}
    </>
  );
}

function FindingCard({ finding }: { finding: OverviewAttentionItem }) {
  return (
    <Panel as="article" className="health-finding-card">
      <div className="health-finding-card__icon" aria-hidden="true">
        {finding.tone === "healthy"
          ? "✓"
          : finding.tone === "update"
            ? "↻"
            : finding.tone === "danger"
              ? "×"
              : finding.tone === "warning"
                ? "!"
                : "?"}
      </div>
      <div className="health-finding-card__body">
        <div className="health-finding-card__title">
          <div>
            <strong>{finding.name}</strong>
            <span>{finding.creator}</span>
          </div>
          <StatusBadge tone={finding.tone}>{finding.badge}</StatusBadge>
        </div>
        <p>{finding.detail}</p>
        <div className="health-finding-card__footer">
          <span>Evidence from the current local scan / registry health state</span>
          <button type="button">Review details</button>
        </div>
      </div>
    </Panel>
  );
}

function RecoveryPanel() {
  return (
    <div className="health-recovery-grid">
      <Panel className="recovery-card recovery-card--primary">
        <span className="section-kicker">RECOVERY</span>
        <h2>Restore points belong to actions, not a separate backup product.</h2>
        <p>
          Sims Mod Health keeps app-controlled restore points for supported update operations.
          It does not present itself as a full Sims save-backup system.
        </p>
      </Panel>

      <Panel className="recovery-card">
        <span className="section-kicker">BEFORE MUTATION</span>
        <h3>Verified snapshot</h3>
        <p>
          The original artifact is copied to app data and its SHA-256 must match before mutation proceeds.
        </p>
      </Panel>

      <Panel className="recovery-card">
        <span className="section-kicker">INTERRUPTED UPDATE</span>
        <h3>Journal retained</h3>
        <p>
          Non-terminal update transactions are marked interrupted on startup; their restore points are retained.
        </p>
      </Panel>

      <Panel className="recovery-card">
        <span className="section-kicker">ROLLBACK</span>
        <h3>Safe overwrite policy</h3>
        <p>
          Rollback refuses to overwrite a target that changed independently after the update.
        </p>
      </Panel>
    </div>
  );
}

function Metric({
  label,
  value,
  tone
}: {
  label: string;
  value: number;
  tone: StatusTone;
}) {
  return (
    <article className={"health-metric health-metric--" + tone}>
      <StatusBadge tone={tone}>{label}</StatusBadge>
      <strong>{value}</strong>
    </article>
  );
}

function filterFindings(
  findings: OverviewAttentionItem[],
  tab: HealthTab
): OverviewAttentionItem[] {
  if (tab === "updates") {
    return findings.filter(
      (item) =>
        item.badge.toLocaleLowerCase().includes("update") ||
        item.tone === "update"
    );
  }

  if (tab === "conflicts") {
    return findings.filter((item) => {
      const badge = item.badge.toLocaleLowerCase();
      const detail = item.detail.toLocaleLowerCase();
      return (
        item.tone === "warning" ||
        item.tone === "danger" ||
        badge.includes("conflict") ||
        badge.includes("duplicate") ||
        badge.includes("dependency") ||
        detail.includes("incompatib") ||
        detail.includes("duplicate")
      );
    });
  }

  return findings;
}

function tabLabel(tab: HealthTab) {
  if (tab === "updates") return "Updates";
  if (tab === "conflicts") return "Conflicts";
  if (tab === "diagnostics") return "Diagnostics";
  if (tab === "recovery") return "Recovery";
  return "All findings";
}

function healthHeading(tab: HealthTab) {
  if (tab === "updates") return "Review update evidence before changing files";
  if (tab === "conflicts") return "Resolve deterministic problems before potential interactions";
  return "Priority health findings";
}

function emptyTitle(tab: HealthTab) {
  if (tab === "updates") return "No update finding is currently surfaced";
  if (tab === "conflicts") return "No conflict finding is currently surfaced";
  return "Nothing needs immediate review";
}

function emptyCopy(tab: HealthTab, snapshot: OverviewSnapshot) {
  if (!snapshot.hasInstallation) {
    return "Run a local scan first. Health does not invent findings without an indexed Sims 4 installation.";
  }
  if (snapshot.registryState === "offline") {
    return "Local evidence remains available while registry-backed health evidence is offline.";
  }
  if (tab === "updates") {
    return "The current evidence set does not contain an actionable update.";
  }
  if (tab === "conflicts") {
    return "The current evidence set does not contain a duplicate, known incompatibility or potential conflict requiring review.";
  }
  return "The latest scan and available registry evidence do not expose an actionable finding.";
}

function formatPlatform(platform: string) {
  return platform.charAt(0).toUpperCase() + platform.slice(1);
}
