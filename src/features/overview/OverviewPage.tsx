import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge } from "../../components/ui/StatusBadge";
import { GameInstallationSummary } from "../game-content/GameInstallationSummary";
import {
  gameContentGateway,
  type GameContentGateway
} from "../game-content/gameContent.gateway";
import { gameContentAttentionCount } from "../game-content/gameContent.presenter";
import type { GameContentHealthSnapshot } from "../game-content/gameContent.types";
import { overviewGateway, type OverviewGateway } from "./overview.gateway";
import type { OverviewSnapshot, ScanProgress } from "./overview.types";
import { useI18n } from "../../i18n/i18n";

type OverviewPageProps = {
  gateway?: OverviewGateway;
  contentGateway?: GameContentGateway;
};

export function OverviewPage({
  gateway = overviewGateway,
  contentGateway = gameContentGateway
}: OverviewPageProps) {
  const { t, tx } = useI18n();
  const [data, setData] = useState<OverviewSnapshot | null>(null);
  const [gameContent, setGameContent] = useState<GameContentHealthSnapshot | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [scanning, setScanning] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);

  const refresh = async () => {
    const [overview, content] = await Promise.all([
      gateway.load(),
      contentGateway.loadHealth()
    ]);
    setData(overview);
    setGameContent(content);
  };

  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;

    Promise.all([gateway.load(), contentGateway.loadHealth()]).then(
      ([snapshot, content]) => {
        if (active) {
          setData(snapshot);
          setGameContent(content);
        }
      }
    );

    gateway.subscribeProgress((next) => {
      if (active) {
        setProgress(next);
        setScanning(true);
      }
    }).then((unlisten) => {
      if (active) {
        stop = unlisten;
      } else {
        unlisten();
      }
    });

    return () => {
      active = false;
      stop?.();
    };
  }, [gateway, contentGateway]);

  const runScan = async () => {
    setScanning(true);
    setScanError(null);
    setProgress(null);

    try {
      const started = data?.hasInstallation
        ? (await gateway.scanCurrent(), true)
        : await gateway.scanSelected();

      if (started) {
        await refresh();
      }
    } catch (error) {
      setScanError(error instanceof Error ? error.message : String(error));
    } finally {
      setScanning(false);
      setProgress(null);
    }
  };

  if (!data) {
    return <OverviewLoading />;
  }

  const liveProgress = progress ?? {
    filesSeen: data.scan.filesSeen,
    filesHashed: data.scan.filesHashed,
    filesSkipped: data.scan.filesSkipped,
    observations: data.scan.observations
  };
  const scanPercent = scanning
    ? liveProgress.filesSeen === 0
      ? 8
      : Math.min(94, Math.max(12, Math.round((liveProgress.filesHashed + liveProgress.filesSkipped) / liveProgress.filesSeen * 100)))
    : data.scan.status === "completed"
      ? 100
      : 24;

  const headline = tx(overviewHeadline(data));
  const unifiedAttention = data.attentionCount + gameContentAttentionCount(gameContent);
  const attentionCopy =
    unifiedAttention === 0
      ? t("No actionable findings are present in the current evidence.")
      : t("{{count}} findings need review across the game, packs and mods.", { count: unifiedAttention });

  return (
    <>
      <Topbar
        gameVersion={data.gameVersion ? t("Patch {{version}}", { version: data.gameVersion }) : t("Patch unknown")}
        platform={formatPlatform(data.platform)}
        indexedCount={data.indexedCount}
        onScan={runScan}
        scanning={scanning}
        scanDisabled={false}
      />

      <OverviewStateBanner data={data} scanning={scanning} scanError={scanError} />

      {!data.hasInstallation ? (
        <Panel className="overview-empty" aria-labelledby="overview-empty-title">
          <div className="overview-empty__icon" aria-hidden="true">⌁</div>
          <div>
            <span className="section-kicker">{t("No local scan yet")}</span>
            <h1 id="overview-empty-title">{t("Scan a Sims 4 installation to build the health view.")}</h1>
            <p>
              {t("The Overview does not invent health numbers. Choose your real Mods folder and the desktop scanner will index it locally.")}
            </p>
            <Button onClick={runScan} disabled={scanning}>
              {scanning ? t("Scanning…") : t("Choose Mods folder and scan")}
            </Button>
          </div>
        </Panel>
      ) : (
        <>
          <section className="page-heading" aria-labelledby="overview-title">
            <div>
              <p className="eyebrow">{t("LIBRARY HEALTH")}</p>
              <h1 id="overview-title">{headline}</h1>
              <p className="lede">{attentionCopy}</p>
            </div>

            <div
              className="health-score"
              aria-label={
                data.healthScore === null
                  ? t("Overall health unavailable")
                  : t("Overall health {{score}} percent", { score: data.healthScore })
              }
              title={data.healthScoreExplanation}
            >
              <div
                className={"score-ring" + (data.healthScore === null ? " score-ring--unknown" : "")}
                aria-hidden="true"
                style={
                  {
                    "--health-score": data.healthScore === null ? "0%" : data.healthScore + "%"
                  } as CSSProperties
                }
              >
                <span>{data.healthScore ?? "—"}</span>
                {data.healthScore !== null && <small>%</small>}
              </div>
              <div>
                <strong>{t("Verified patch health")}</strong>
                <span>{tx(scanLabel(data))}</span>
              </div>
            </div>
          </section>

          <details className="health-explanation">
            <summary>{t("How this score is calculated")}</summary>
            <p>{data.healthScoreExplanation}</p>
          </details>

          <GameInstallationSummary
            health={gameContent}
            modCount={data.indexedCount}
            fallbackVersion={data.gameVersion}
          />

          <section className="stat-grid" aria-label={t("Mod health summary")}>
            <Stat label={t("Healthy")} value={data.healthCounts.healthy} tone="healthy" />
            <Stat label={t("Updates")} value={data.healthCounts.updates} tone="update" />
            <Stat label={t("Conflicts")} value={data.healthCounts.conflicts} tone="warning" />
            <Stat label={t("Unknown")} value={data.healthCounts.unknown} tone="muted" />
          </section>

          <section className="content-grid">
            <Panel as="article" className="attention-panel">
              <div className="panel-header">
                <div>
                  <span className="section-kicker">{t("Needs attention")}</span>
                  <h2>{t("Review the most actionable findings first")}</h2>
                </div>
                <Button variant="text">{t("View all")}</Button>
              </div>

              {data.attention.length === 0 ? (
                <div className="attention-empty">
                  <span aria-hidden="true">✓</span>
                  <div>
                    <strong>{t("No actionable findings")}</strong>
                    <p>{t("Nothing in the current scan and health evidence needs immediate review.")}</p>
                  </div>
                </div>
              ) : (
                <div className="attention-list">
                  {data.attention.map((item, index) => (
                    <button className="attention-row" key={item.name + "-" + item.badge + "-" + index}>
                      <div className={"item-avatar item-avatar--" + item.tone} aria-hidden="true">
                        {item.name.slice(0, 2).toUpperCase()}
                      </div>
                      <div className="item-copy">
                        <strong>{item.name}</strong>
                        <span>{item.creator} · {item.detail}</span>
                      </div>
                      <StatusBadge tone={item.tone}>{tx(item.badge)}</StatusBadge>
                      <span className="row-arrow" aria-hidden="true">›</span>
                    </button>
                  ))}
                </div>
              )}
            </Panel>

            <Panel as="aside" className="scan-panel" aria-labelledby="installation-title">
              <div className="scan-title-row">
                <div>
                  <span className="section-kicker">{t("Current installation")}</span>
                  <h2 id="installation-title">{t("{{count}} items indexed", { count: data.indexedCount })}</h2>
                </div>
                <span className={"scan-status scan-status--" + data.scan.status}>
                  {scanning ? t("Scanning") : tx(formatScanStatus(data.scan.status))}
                </span>
              </div>

              <div
                className="scan-meter"
                role="progressbar"
                aria-label={t("Scan progress")}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={scanPercent}
              >
                <span style={{ width: scanPercent + "%" }} />
              </div>

              {scanning && (
                <p className="scan-progress-copy" role="status">
                  {t("{{seen}} seen · {{hashed}} hashed · {{skipped}} unchanged", { seen: liveProgress.filesSeen, hashed: liveProgress.filesHashed, skipped: liveProgress.filesSkipped })}
                </p>
              )}

              <dl className="scan-facts">
                <div><dt>{t("Script mods")}</dt><dd>{data.installation.scriptMods}</dd></div>
                <div><dt>{t("Package / CC files")}</dt><dd>{data.installation.packageFiles}</dd></div>
                <div>
                  <dt>{t("Unidentified files")}</dt>
                  <dd>{data.installation.unidentified ?? "—"}</dd>
                </div>
                <div><dt>{t("Exact duplicate groups")}</dt><dd>{data.installation.exactDuplicates}</dd></div>
              </dl>
              <Button onClick={runScan} disabled={scanning}>
                {scanning ? t("Scanning…") : t("Run incremental scan")}
              </Button>
            </Panel>
          </section>

          <Panel className="recommendation-strip" aria-labelledby="discover-title">
            <div className="recommendation-icon" aria-hidden="true">✦</div>
            <div>
              <span className="section-kicker">{t("Discover")}</span>
              <h2 id="discover-title">{t("Recommendations stay separate from health evidence.")}</h2>
              <p>
                {t("Discover will use resolved library data only after compatibility and known-conflict filters are applied.")}
              </p>
            </div>
            <Button>{t("Open Discover")}</Button>
          </Panel>
        </>
      )}
    </>
  );
}

function OverviewLoading() {
  const { t } = useI18n();
  return (
    <section className="overview-loading" role="status" aria-live="polite">
      <span className="overview-loading__pulse" aria-hidden="true" />
      <span>{t("Loading current installation health…")}</span>
    </section>
  );
}

function OverviewStateBanner({
  data,
  scanning,
  scanError
}: {
  data: OverviewSnapshot;
  scanning: boolean;
  scanError: string | null;
}) {
  const { t, tx } = useI18n();
  const notices = useMemo(() => {
    const result: { tone: string; title: string; detail: string }[] = [];

    if (scanError) {
      result.push({
        tone: "danger",
        title: "Scan could not complete",
        detail: scanError
      });
    }

    if (scanning || data.scan.status === "running") {
      result.push({
        tone: "update",
        title: "Scan in progress",
        detail: "Local counts are updating. Registry health refreshes after the scan completes."
      });
    } else if (data.scan.stale) {
      result.push({
        tone: "muted",
        title: "Scan data is stale",
        detail: "The latest completed scan is older than 24 hours. Local data remains visible until you rescan."
      });
    } else if (data.scan.partial) {
      result.push({
        tone: "warning",
        title: "Partial local results",
        detail: "The latest scan contains recoverable observations or did not complete cleanly."
      });
    }

    if (data.registryState === "offline") {
      result.push({
        tone: "muted",
        title: "Registry offline",
        detail: data.registryDetail
      });
    } else if (data.registryState === "partial") {
      result.push({
        tone: "warning",
        title: "Registry data is partial",
        detail: data.registryDetail
      });
    }

    return result;
  }, [data, scanning, scanError]);

  if (notices.length === 0) return null;

  return (
    <div className="overview-notices" aria-label={t("Overview data status")}>
      {notices.map((notice) => (
        <div className={"overview-notice overview-notice--" + notice.tone} key={notice.title}>
          <span aria-hidden="true">{notice.tone === "danger" ? "×" : notice.tone === "update" ? "↻" : "!"}</span>
          <div>
            <strong>{tx(notice.title)}</strong>
            <p>{tx(notice.detail)}</p>
          </div>
        </div>
      ))}
    </div>
  );
}

function Stat({
  label,
  value,
  tone
}: {
  label: string;
  value: number;
  tone: "healthy" | "update" | "warning" | "muted";
}) {
  return (
    <article className="stat-card">
      <StatusBadge tone={tone}>{label}</StatusBadge>
      <strong className="stat-card__value">{value}</strong>
    </article>
  );
}

function overviewHeadline(data: OverviewSnapshot) {
  if (data.registryState === "offline") {
    return "Local scan is available while the registry is offline.";
  }
  if (data.healthScore === null) {
    return "Health evidence is incomplete.";
  }
  if (data.healthScore >= 90) {
    return "Most installed items have verified patch compatibility.";
  }
  if (data.healthScore >= 70) {
    return "Some installed items need review.";
  }
  return "Review compatibility findings before the next session.";
}

function scanLabel(data: OverviewSnapshot) {
  if (data.scan.status === "running") return "Scan in progress";
  if (data.scan.stale) return "Latest scan is older than 24h";
  if (data.scan.completedAt) return "Current scan completed";
  return "No completed scan";
}

function formatPlatform(platform: string) {
  return platform.charAt(0).toUpperCase() + platform.slice(1);
}

function formatScanStatus(status: string) {
  if (status === "completed") return "Current";
  if (status === "cancelled") return "Cancelled";
  if (status === "failed") return "Failed";
  if (status === "empty") return "No scan";
  return status;
}
