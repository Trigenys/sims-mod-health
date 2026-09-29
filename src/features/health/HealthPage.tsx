import { useEffect, useMemo, useState } from "react";
import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge, type StatusTone } from "../../components/ui/StatusBadge";
import { DiagnosticsPage } from "../diagnostics/DiagnosticsPage";
import { GameContentDrawer } from "../game-content/GameContentDrawer";
import {
  gameContentGateway,
  type GameContentGateway
} from "../game-content/gameContent.gateway";
import {
  gameContentBadge,
  gameContentTone,
  localizedGameContentReason,
  isActionableGameContent,
  isUpdateGameContent,
  packHealthSummary
} from "../game-content/gameContent.presenter";
import type {
  GameContentHealthFinding,
  GameContentHealthSnapshot,
  ProviderUpdateCapability,
  ProviderUpdateSession
} from "../game-content/gameContent.types";
import { overviewGateway, type OverviewGateway } from "../overview/overview.gateway";
import type {
  OverviewAttentionItem,
  OverviewSnapshot,
  PotentialConflictGroup
} from "../overview/overview.types";
import {
  localizeAttentionBadge,
  localizeAttentionCreator,
  localizeOverviewDetail
} from "../overview/overview.i18n";
import { useI18n } from "../../i18n/i18n";

export type HealthTab =
  | "all"
  | "updates"
  | "conflicts"
  | "diagnostics"
  | "recovery";

type UpdateScope = "all" | "game" | "mods";

type HealthPageProps = {
  gateway?: OverviewGateway;
  contentGateway?: GameContentGateway;
  initialTab?: HealthTab;
  onOpenLibrary?: () => void;
};

type UnifiedFinding =
  | {
      id: string;
      kind: "mod";
      label: "Mod";
      name: string;
      creator: string;
      detail: string;
      badge: string;
      tone: StatusTone;
      priority: number;
      mod: OverviewAttentionItem;
    }
  | {
      id: string;
      kind: "game" | "pack";
      label: "Game" | "Pack";
      name: string;
      creator: string;
      detail: string;
      badge: string;
      tone: StatusTone;
      priority: number;
      content: GameContentHealthFinding;
    }
  | {
      id: string;
      kind: "conflictGroup";
      label: "Possible interaction";
      name: string;
      creator: string;
      detail: string;
      badge: string;
      tone: StatusTone;
      priority: number;
      conflict: PotentialConflictGroup;
    };

const tabs: { value: HealthTab; label: string }[] = [
  { value: "all", label: "Everything to review" },
  { value: "updates", label: "Updates" },
  { value: "conflicts", label: "Conflicts" },
  { value: "diagnostics", label: "Diagnostics" },
  { value: "recovery", label: "Recovery" }
];

export function HealthPage({
  gateway = overviewGateway,
  contentGateway = gameContentGateway,
  initialTab = "all",
  onOpenLibrary
}: HealthPageProps) {
  const { t, tx } = useI18n();
  const [snapshot, setSnapshot] = useState<OverviewSnapshot | null>(null);
  const [contentHealth, setContentHealth] = useState<GameContentHealthSnapshot | null>(null);
  const [capability, setCapability] = useState<ProviderUpdateCapability | null>(null);
  const [selectedContent, setSelectedContent] = useState<GameContentHealthFinding | null>(null);
  const [selectedConflict, setSelectedConflict] = useState<PotentialConflictGroup | null>(null);
  const [providerSession, setProviderSession] = useState<ProviderUpdateSession | null>(null);
  const [providerBusy, setProviderBusy] = useState(false);
  const [providerError, setProviderError] = useState<string | null>(null);
  const [tab, setTab] = useState<HealthTab>(initialTab);
  const [updateScope, setUpdateScope] = useState<UpdateScope>("all");

  useEffect(() => {
    let active = true;

    Promise.all([
      gateway.load(),
      contentGateway.loadHealth(),
      contentGateway.loadCapability()
    ]).then(([overview, content, providerCapability]) => {
      if (!active) return;
      setSnapshot(overview);
      setContentHealth(content);
      setCapability(providerCapability);

      const visual = new URLSearchParams(window.location.search).get("visual");
      if (visual === "health-pack") {
        setTab("updates");
        setSelectedContent(
          content.packs.find((finding) => isActionableGameContent(finding))
            ?? content.packs[0]
            ?? null
        );
      }
    });

    return () => {
      active = false;
    };
  }, [gateway, contentGateway]);

  const allFindings = useMemo(
    () => buildUnifiedFindings(snapshot?.attention ?? [], contentHealth, t),
    [snapshot, contentHealth, t]
  );

  const potentialConflicts = useMemo(
    () => buildPotentialConflictFindings(
      snapshot?.conflictAggregation.potentialConflictGroups ?? [],
      t
    ),
    [snapshot, t]
  );

  const findings = useMemo(
    () => filterFindings(allFindings, tab, updateScope, potentialConflicts),
    [allFindings, tab, updateScope, potentialConflicts]
  );

  if (!snapshot) {
    return (
      <section className="health-loading" role="status">
        {t("Loading current health evidence…")}
      </section>
    );
  }

  const packs = packHealthSummary(contentHealth);
  const updateCount = allFindings.filter(isUnifiedUpdate).length;
  const combinedAttention = allFindings.length;
  const groupedConflictCount =
    snapshot.healthCounts.conflicts + snapshot.conflictAggregation.potentialConflictGroupCount;

  const startProviderUpdate = async (finding: GameContentHealthFinding) => {
    setProviderBusy(true);
    setProviderError(null);
    try {
      const session = await contentGateway.startProviderUpdate(
        finding.kind,
        finding.targetId
      );
      setProviderSession(session);
    } catch (error) {
      setProviderError(error instanceof Error ? error.message : String(error));
    } finally {
      setProviderBusy(false);
    }
  };

  const verifyProviderUpdate = async () => {
    if (!providerSession) return;
    setProviderBusy(true);
    setProviderError(null);
    try {
      const result = await contentGateway.verifyProviderUpdate(providerSession.id);
      setProviderSession(result.session);
      setContentHealth(result.gameContentHealth);
      setSnapshot(await gateway.load());
      if (selectedContent) {
        const refreshed = [
          result.gameContentHealth.game,
          ...result.gameContentHealth.packs
        ].find(
          (finding) =>
            finding?.kind === selectedContent.kind
            && finding.targetId === selectedContent.targetId
        );
        setSelectedContent(refreshed ?? selectedContent);
      }
    } catch (error) {
      setProviderError(error instanceof Error ? error.message : String(error));
    } finally {
      setProviderBusy(false);
    }
  };

  return (
    <>
      <Topbar
        gameVersion={
          contentHealth?.game?.currentVersion
            ? t("Patch {{version}}", { version: contentHealth.game.currentVersion })
            : snapshot.gameVersion
              ? t("Patch {{version}}", { version: snapshot.gameVersion })
              : t("Game version not detected yet")
        }
        platform={formatPlatform(snapshot.platform)}
        indexedCount={snapshot.indexedCount}
      />

      <section className="health-hero" aria-labelledby="health-title">
        <div>
          <p className="eyebrow">{t("GAME & MOD HEALTH")}</p>
          <h1 id="health-title">{t("See what needs your attention.")}</h1>
          <p className="lede">
            {t("We combine your game, packs and mods so you can fix the most important things first.")}
          </p>
        </div>

        <div className="health-hero__actions">
          <div className="health-snapshot">
            <span>{t("Needs attention")}</span>
            <strong>{combinedAttention}</strong>
          </div>
          <Button variant="primary" onClick={() => setTab("updates")}>
            {t("Review updates")}
          </Button>
        </div>
      </section>

      <section className="health-metrics" aria-label={t("Game, packs and mods summary")}>
        <Metric label={t("Needs attention")} value={combinedAttention} tone="danger" />
        <Metric label={t("Updates")} value={updateCount} tone="update" />
        <Metric label={t("Conflicts")} value={snapshot.healthCounts.conflicts} tone="warning" />
        <Metric label={t("Not identified yet")} value={countUnknown(allFindings)} tone="muted" />
        <Metric
          label={t("Healthy packs")}
          value={packs.current}
          tone="healthy"
        />
      </section>

      <nav className="health-tabs" aria-label={t("Health views")}>
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
              {t(item.label as "Everything to review" | "Updates" | "Conflicts" | "Diagnostics" | "Recovery")}
              {item.value === "updates" && updateCount > 0 && <span>{updateCount}</span>}
              {item.value === "conflicts" && groupedConflictCount > 0 && (
                <span>{groupedConflictCount}</span>
              )}
            </button>
          );
        })}
      </nav>

      {tab === "diagnostics" ? (
        <section className="health-diagnostics" aria-label={t("DIAGNOSTIC EVIDENCE")}>
          <DiagnosticsPage embedded />
        </section>
      ) : tab === "recovery" ? (
        <RecoveryPanel />
      ) : (
        <>
          {tab === "updates" && (
            <div className="health-update-toolbar">
              <div className="health-update-scopes" aria-label={t("Update types")}>
                {([
                  ["all", t("All")],
                  ["game", t("Game & packs")],
                  ["mods", t("Mods")]
                ] as const).map(([value, label]) => (
                  <button
                    key={value}
                    type="button"
                    className={
                      updateScope === value
                        ? "health-update-scope health-update-scope--active"
                        : "health-update-scope"
                    }
                    aria-pressed={updateScope === value}
                    onClick={() => setUpdateScope(value)}
                  >
                    {label}
                  </button>
                ))}
              </div>

              {packs.total > 0 && (
                <div className="healthy-pack-summary" role="status">
                  <span aria-hidden="true">✓</span>
                  <strong>{t("{{count}} packs current", { count: packs.current })}</strong>
                  {packs.attention > 0 && <span>{t("{{count}} need review", { count: packs.attention })}</span>}
                </div>
              )}
            </div>
          )}

          <div className={selectedContent || selectedConflict ? "health-layout health-layout--drawer" : "health-layout"}>
            <section className="health-findings" aria-label={tx(tabLabel(tab))}>
              <div className="health-section-heading">
                <div>
                  <span className="section-kicker">{tx(tabLabel(tab)).toUpperCase()}</span>
                  <h2>{tx(healthHeading(tab))}</h2>
                </div>
                <span>{t("{{count}} items shown", { count: findings.length })}</span>
              </div>

              {findings.length === 0 ? (
                <Panel className="health-empty">
                  <div className="health-empty__icon" aria-hidden="true">✓</div>
                  <div>
                    <h3>{tx(emptyTitle(tab))}</h3>
                    <p>{tx(emptyCopy(tab, snapshot))}</p>
                  </div>
                </Panel>
              ) : (
                <div className="health-finding-list">
                  {findings.map((finding) => (
                    <FindingCard
                      finding={finding}
                      key={finding.id}
                      onReview={() => {
                        if (finding.kind === "mod") {
                          onOpenLibrary?.();
                          return;
                        }
                        if (finding.kind === "conflictGroup") {
                          setSelectedContent(null);
                          setSelectedConflict(finding.conflict);
                          return;
                        }
                        setSelectedConflict(null);
                        setSelectedContent(finding.content);
                        setProviderError(null);
                      }}
                    />
                  ))}
                </div>
              )}
            </section>

            {selectedContent ? (
              <GameContentDrawer
                finding={selectedContent}
                capability={capability}
                session={providerSession}
                busy={providerBusy}
                error={providerError}
                onClose={() => setSelectedContent(null)}
                onUpdate={() => void startProviderUpdate(selectedContent)}
                onVerify={() => void verifyProviderUpdate()}
              />
            ) : selectedConflict ? (
              <ConflictEvidenceDrawer
                group={selectedConflict}
                onClose={() => setSelectedConflict(null)}
              />
            ) : (
              <aside className="health-guardrails">
                <Panel>
                  <span className="section-kicker">{t("SAFE CHANGES")}</span>
                  <h2>{t("Changes can be undone.")}</h2>
                  <p>
                    {t("Before changing supported mods, Sims Mod Health keeps a restore point. Game and pack updates stay with EA app or Steam.")}
                  </p>
                  <ul>
                    <li><span>✓</span> {t("Restore point before supported mod updates")}</li>
                    <li><span>✓</span> {t("SHA-256 integrity checks")}</li>
                    <li><span>✓</span> {t("Game updates are checked again after you return")}</li>
                    <li><span>✓</span> {t("Unknown does not mean broken")}</li>
                  </ul>
                  <button
                    type="button"
                    className="health-link"
                    onClick={() => setTab("recovery")}
                  >
                    {t("Open Recovery")}
                    <span aria-hidden="true">→</span>
                  </button>
                </Panel>

                <Panel className="health-safety">
                  <span className="section-kicker">{t("HOW WE HANDLE UNCERTAINTY")}</span>
                  <h2>{t("Not enough information does not mean broken.")}</h2>
                  <p>
                    {t("If we cannot confirm something, we show that clearly instead of guessing.")}
                  </p>
                </Panel>
              </aside>
            )}
          </div>
        </>
      )}
    </>
  );
}

function FindingCard({
  finding,
  onReview
}: {
  finding: UnifiedFinding;
  onReview: () => void;
}) {
  const { t, tx } = useI18n();
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
            <span className={"finding-kind finding-kind--" + finding.kind}>
              {tx(finding.label)}
            </span>
            <strong>{finding.name}</strong>
            <span>{tx(finding.creator)}</span>
          </div>
          <StatusBadge tone={finding.tone}>{tx(finding.badge)}</StatusBadge>
        </div>
        <p>{tx(finding.detail)}</p>
        <div className="health-finding-card__footer">
          <span>
            {finding.kind === "mod"
              ? t("Checked from your Mods folder and available online mod information")
              : finding.kind === "conflictGroup"
                ? t("Grouped from files that may change the same game content")
                : t("Checked from your installed game and pack information")}
          </span>
          <button type="button" onClick={onReview}>
            {finding.kind === "mod"
              ? t("Review in Library")
              : finding.kind === "conflictGroup"
                ? t("Review evidence")
                : t("Review details")}
          </button>
        </div>
      </div>
    </Panel>
  );
}

function buildUnifiedFindings(
  modFindings: OverviewAttentionItem[],
  content: GameContentHealthSnapshot | null,
  t: ReturnType<typeof useI18n>["t"]
): UnifiedFinding[] {
  const mods: UnifiedFinding[] = modFindings.map((finding, index) => ({
    id: "mod-" + finding.name + "-" + index,
    kind: "mod",
    label: "Mod",
    name: finding.name,
    creator: localizeAttentionCreator(finding.creator, t),
    detail: localizeOverviewDetail(finding.detail, t),
    badge: localizeAttentionBadge(finding.badge, t),
    tone: finding.tone,
    priority: modPriority(finding),
    mod: finding
  }));

  const gameContent = [content?.game ?? null, ...(content?.packs ?? [])]
    .filter((finding): finding is GameContentHealthFinding => finding !== null)
    .filter(isActionableGameContent)
    .map<UnifiedFinding>((finding) => ({
      id: finding.kind + "-" + finding.targetId,
      kind: finding.kind,
      label: finding.kind === "game" ? "Game" : "Pack",
      name: finding.kind === "game" ? "The Sims 4" : finding.targetId,
      creator: finding.disputed
        ? t("Compatibility sources disagree")
        : t("Game and pack check"),
      detail: localizedGameContentReason(finding, t),
      badge: gameContentBadge(finding.state),
      tone: gameContentTone(finding.state),
      priority: contentPriority(finding),
      content: finding
    }));

  return [...gameContent, ...mods].sort(
    (left, right) => right.priority - left.priority || left.name.localeCompare(right.name)
  );
}

function buildPotentialConflictFindings(
  groups: PotentialConflictGroup[],
  t: ReturnType<typeof useI18n>["t"]
): UnifiedFinding[] {
  return groups.map((group, index) => ({
    id: "conflict-group-" + index,
    kind: "conflictGroup" as const,
    label: "Possible interaction" as const,
    name: t("Possible interaction across {{count}} files", {
      count: group.relativePaths.length
    }),
    creator: t("Needs more evidence"),
    detail: t("{{files}} files may affect some of the same game content. We grouped {{pairs}} file-pair observations into one item instead of showing each pair separately.", {
      files: group.relativePaths.length,
      pairs: group.overlapPairCount
    }),
    badge: t("Low confidence"),
    tone: "muted" as StatusTone,
    priority: 20,
    conflict: group
  }));
}

function filterFindings(
  findings: UnifiedFinding[],
  tab: HealthTab,
  scope: UpdateScope,
  potentialConflicts: UnifiedFinding[]
): UnifiedFinding[] {
  if (tab === "updates") {
    return findings.filter((finding) => {
      if (!isUnifiedUpdate(finding)) return false;
      if (scope === "game") return finding.kind !== "mod";
      if (scope === "mods") return finding.kind === "mod";
      return true;
    });
  }

  if (tab === "conflicts") {
    const actionable = findings.filter((finding) => {
      if (finding.kind !== "mod") return false;
      const badge = finding.badge.toLocaleLowerCase();
      const detail = finding.detail.toLocaleLowerCase();
      return (
        finding.tone === "warning"
        || finding.tone === "danger"
        || badge.includes("conflict")
        || badge.includes("duplicate")
        || badge.includes("dependency")
        || detail.includes("incompatib")
        || detail.includes("duplicate")
      );
    });
    return [...actionable, ...potentialConflicts];
  }

  return findings;
}

function isUnifiedUpdate(finding: UnifiedFinding) {
  if (finding.kind === "mod") {
    return finding.tone === "update" || finding.badge.toLocaleLowerCase().includes("update");
  }
  if (finding.kind === "conflictGroup") {
    return false;
  }
  return isUpdateGameContent(finding.content);
}

function contentPriority(finding: GameContentHealthFinding) {
  if (finding.state === "game_update_required") return 100;
  if (finding.state === "update_available") return 95;
  if (finding.state === "local_integrity_uncertain") return 85;
  if (finding.disputed) return 80;
  if (finding.state === "metadata_stale") return 45;
  return 35;
}

function modPriority(finding: OverviewAttentionItem) {
  if (finding.tone === "danger") return 90;
  if (finding.tone === "update") return 75;
  if (finding.tone === "warning") return 65;
  return 30;
}

function countUnknown(findings: UnifiedFinding[]) {
  return findings.filter(
    (finding) =>
      finding.tone === "muted"
      || finding.badge.toLocaleLowerCase().includes("unknown")
      || finding.badge.toLocaleLowerCase().includes("stale")
  ).length;
}

function ConflictEvidenceDrawer({
  group,
  onClose
}: {
  group: PotentialConflictGroup;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const visibleFiles = group.relativePaths.slice(0, 8);
  const remainingFiles = Math.max(0, group.relativePaths.length - visibleFiles.length);

  return (
    <Panel
      as="aside"
      className="conflict-evidence-drawer"
      role="dialog"
      aria-label={t("Possible interaction evidence")}
    >
      <div className="conflict-evidence-drawer__header">
        <div>
          <span className="section-kicker">{t("WHY THESE FILES WERE GROUPED")}</span>
          <h2>{t("Possible interaction across {{count}} files", { count: group.relativePaths.length })}</h2>
        </div>
        <button type="button" onClick={onClose} aria-label={t("Close evidence")}>×</button>
      </div>

      <StatusBadge tone="muted">{t("Low confidence")}</StatusBadge>
      <p className="conflict-evidence-drawer__intro">
        {t("These files may change some of the same game content. That can be intentional, so this group does not increase Needs attention unless stronger evidence appears.")}
      </p>

      <dl className="conflict-evidence-facts">
        <div>
          <dt>{t("Files in group")}</dt>
          <dd>{group.relativePaths.length}</dd>
        </div>
        <div>
          <dt>{t("File-pair observations")}</dt>
          <dd>{group.overlapPairCount}</dd>
        </div>
        <div>
          <dt>{t("Shared game resources")}</dt>
          <dd>{group.sharedResourceCount}</dd>
        </div>
      </dl>

      <section className="conflict-evidence-section">
        <h3>{t("Files")}</h3>
        <ul>
          {visibleFiles.map((path) => <li key={path}>{path}</li>)}
        </ul>
        {remainingFiles > 0 && (
          <p>{t("+{{count}} more files in this group", { count: remainingFiles })}</p>
        )}
      </section>

      <section className="conflict-evidence-section">
        <h3>{t("Sample evidence")}</h3>
        {group.sampleOverlapPairs.length === 0 ? (
          <p>{t("No file-pair sample is available in this preview.")}</p>
        ) : (
          <ul>
            {group.sampleOverlapPairs.map((pair, index) => (
              <li key={pair.leftFileId + "-" + pair.rightFileId + "-" + index}>
                <strong>{pair.leftRelativePath}</strong>
                <span>↔</span>
                <strong>{pair.rightRelativePath}</strong>
                <small>
                  {t("{{count}} shared game resources", { count: pair.sharedResourceCount })}
                </small>
              </li>
            ))}
          </ul>
        )}
      </section>

      <p className="conflict-evidence-drawer__footnote">
        {t("The app keeps the underlying file-pair evidence, but this view groups it so one cluster does not look like dozens or thousands of separate broken mods.")}
      </p>
    </Panel>
  );
}

function RecoveryPanel() {
  const { t } = useI18n();
  return (
    <div className="health-recovery-grid">
      <Panel className="recovery-card recovery-card--primary">
        <span className="section-kicker">{t("Recovery").toUpperCase()}</span>
        <h2>{t("Restore points are created only when needed.")}</h2>
        <p>
          {t("Before supported mod updates, Sims Mod Health keeps a verified copy. Game and pack updates happen in EA app or Steam and are checked again when you return.")}
        </p>
      </Panel>

      <Panel className="recovery-card">
        <span className="section-kicker">{t("BEFORE A MOD CHANGE")}</span>
        <h3>{t("Verified restore copy")}</h3>
        <p>
          {t("The original file is copied and verified before any supported mod update changes it.")}
        </p>
      </Panel>

      <Panel className="recovery-card">
        <span className="section-kicker">{t("GAME UPDATE")}</span>
        <h3>{t("Verify after return")}</h3>
        <p>
          {t("Opening EA app or Steam is not enough. We check the installed game and packs again when you return.")}
        </p>
      </Panel>

      <Panel className="recovery-card">
        <span className="section-kicker">{t("UNDO")}</span>
        <h3>{t("Do not overwrite newer changes")}</h3>
        <p>
          {t("Undo stops if the target file changed after the update, so newer changes are not overwritten.")}
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

function tabLabel(tab: HealthTab) {
  if (tab === "updates") return "Updates";
  if (tab === "conflicts") return "Conflicts";
  if (tab === "diagnostics") return "Diagnostics";
  if (tab === "recovery") return "Recovery";
  return "Everything to review";
}

function healthHeading(tab: HealthTab) {
  if (tab === "updates") return "Updates found for your game, packs and mods";
  if (tab === "conflicts") return "Fix confirmed problems before possible conflicts";
  return "What needs your attention first";
}

function emptyTitle(tab: HealthTab) {
  if (tab === "updates") return "No updates need your attention";
  if (tab === "conflicts") return "No conflicts need your attention";
  return "Nothing needs immediate action";
}

function emptyCopy(tab: HealthTab, snapshot: OverviewSnapshot) {
  if (!snapshot.hasInstallation) {
    return "Run a scan first so we can check your game and mods.";
  }
  if (snapshot.registryState === "offline") {
    return "Your local scan is still available, but online compatibility checks are temporarily unavailable.";
  }
  if (tab === "updates") {
    return "We did not find any game, pack or mod update that needs action.";
  }
  if (tab === "conflicts") {
    return "We did not find a duplicate or conflict that needs your attention.";
  }
  return "We did not find anything that needs your attention right now.";
}

function formatPlatform(platform: string) {
  return platform.charAt(0).toUpperCase() + platform.slice(1);
}
