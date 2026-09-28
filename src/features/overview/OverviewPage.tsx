import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge } from "../../components/ui/StatusBadge";
import { useI18n } from "../../i18n/I18nProvider";
import { GameInstallationSummary } from "../game-content/GameInstallationSummary";
import {
  gameContentGateway,
  type GameContentGateway
} from "../game-content/gameContent.gateway";
import { gameContentAttentionCount } from "../game-content/gameContent.presenter";
import type { GameContentHealthSnapshot } from "../game-content/gameContent.types";
import { overviewGateway, type OverviewGateway } from "./overview.gateway";
import type { OverviewSnapshot, ScanProgress } from "./overview.types";

type OverviewPageProps = {
  gateway?: OverviewGateway;
  contentGateway?: GameContentGateway;
};

export function OverviewPage({
  gateway = overviewGateway,
  contentGateway = gameContentGateway
}: OverviewPageProps) {
  const { t } = useI18n();
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

  const headline = data.registryState === "offline"
    ? t("overview.headlineOffline")
    : data.healthScore === null
      ? t("overview.headlineIncomplete")
      : data.healthScore >= 90
        ? t("overview.headlineHealthy")
        : data.healthScore >= 70
          ? t("overview.headlineReview")
          : t("overview.headlineWarning");
  const unifiedAttention = data.attentionCount + gameContentAttentionCount(gameContent);
  const attentionCopy =
    unifiedAttention === 0
      ? t("overview.noActionable")
      : t("overview.findingsReview", { count: unifiedAttention });

  return (
    <>
      <Topbar
        gameVersion={data.gameVersion ? "Patch " + data.gameVersion : t("common.patchUnknown")}
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
            <span className="section-kicker">{t("overview.noScan")}</span>
            <h1 id="overview-empty-title">{t("overview.noScanTitle")}</h1>
            <p>{t("overview.noScanCopy")}</p>
            <Button onClick={runScan} disabled={scanning}>
              {scanning ? t("topbar.scanning") : t("overview.chooseMods")}
            </Button>
          </div>
        </Panel>
      ) : (
        <>
          <section className="page-heading" aria-labelledby="overview-title">
            <div>
              <p className="eyebrow">{t("overview.eyebrow")}</p>
              <h1 id="overview-title">{headline}</h1>
              <p className="lede">{attentionCopy}</p>
            </div>

            <div
              className="health-score"
              aria-label={
                data.healthScore === null
                  ? t("overview.overallUnavailable")
                  : t("overview.overallHealth", { score: data.healthScore })
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
                <strong>{t("overview.score")}</strong>
                <span>{data.scan.status === "running" ? t("overview.scanRunning") : data.scan.stale ? t("overview.scanStale") : data.scan.completedAt ? t("overview.scanComplete") : t("overview.noCompletedScan")}</span>
              </div>
            </div>
          </section>

          <details className="health-explanation">
            <summary>{t("overview.scoreHow")}</summary>
            <p>{data.healthScoreExplanation}</p>
          </details>

          <GameInstallationSummary
            health={gameContent}
            modCount={data.indexedCount}
            fallbackVersion={data.gameVersion}
          />

          <section className="stat-grid" aria-label="Mod health summary">
            <Stat label="Healthy" value={data.healthCounts.healthy} tone="healthy" />
            <Stat label="Updates" value={data.healthCounts.updates} tone="update" />
            <Stat label="Conflicts" value={data.healthCounts.conflicts} tone="warning" />
            <Stat label="Unknown" value={data.healthCounts.unknown} tone="muted" />
          </section>

          <section className="content-grid">
            <Panel as="article" className="attention-panel">
              <div className="panel-header">
                <div>
                  <span className="section-kicker">{t("overview.needsAttention")}</span>
                  <h2>{t("overview.reviewFirst")}</h2>
                </div>
                <Button variant="text">{t("overview.viewAll")}</Button>
              </div>

              {data.attention.length === 0 ? (
                <div className="attention-empty">
                  <span aria-hidden="true">✓</span>
                  <div>
                    <strong>{t("overview.noFindings")}</strong>
                    <p>{t("overview.noFindingsCopy")}</p>
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
                      <StatusBadge tone={item.tone}>{item.badge}</StatusBadge>
                      <span className="row-arrow" aria-hidden="true">›</span>
                    </button>
                  ))}
                </div>
              )}
            </Panel>

            <Panel as="aside" className="scan-panel" aria-labelledby="installation-title">
              <div className="scan-title-row">
                <div>
                  <span className="section-kicker">{t("overview.currentInstallation")}</span>
                  <h2 id="installation-title">{t("overview.itemsIndexed", { count: data.indexedCount })}</h2>
                </div>
                <span className={"scan-status scan-status--" + data.scan.status}>
                  {scanning ? t("overview.scanning") : data.scan.status === "completed" ? t("overview.statusCurrent") : data.scan.status === "cancelled" ? t("overview.statusCancelled") : data.scan.status === "failed" ? t("overview.statusFailed") : data.scan.status === "empty" ? t("overview.statusNoScan") : data.scan.status}
                </span>
              </div>

              <div
                className="scan-meter"
                role="progressbar"
                aria-label={t("overview.scanProgress")}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={scanPercent}
              >
                <span style={{ width: scanPercent + "%" }} />
              </div>

              {scanning && (
                <p className="scan-progress-copy" role="status">
                  {t("overview.seenHashed", { seen: liveProgress.filesSeen, hashed: liveProgress.filesHashed, skipped: liveProgress.filesSkipped })}
                </p>
              )}

              <dl className="scan-facts">
                <div><dt>{t("overview.scriptMods")}</dt><dd>{data.installation.scriptMods}</dd></div>
                <div><dt>{t("overview.packageFiles")}</dt><dd>{data.installation.packageFiles}</dd></div>
                <div>
                  <dt>{t("overview.unidentified")}</dt>
                  <dd>{data.installation.unidentified ?? "—"}</dd>
                </div>
                <div><dt>{t("overview.duplicates")}</dt><dd>{data.installation.exactDuplicates}</dd></div>
              </dl>
              <Button onClick={runScan} disabled={scanning}>
                {scanning ? t("topbar.scanning") : t("overview.runIncremental")}
              </Button>
            </Panel>
          </section>

          <Panel className="recommendation-strip" aria-labelledby="discover-title">
            <div className="recommendation-icon" aria-hidden="true">✦</div>
            <div>
              <span className="section-kicker">{t("overview.discover")}</span>
              <h2 id="discover-title">{t("overview.discoverTitle")}</h2>
              <p>{t("overview.discoverCopy")}</p>
            </div>
            <Button>{t("overview.openDiscover")}</Button>
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
      <span>{t("overview.loading")}</span>
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
    <div className="overview-notices" aria-label="Overview data status">
      {notices.map((notice) => (
        <div className={"overview-notice overview-notice--" + notice.tone} key={notice.title}>
          <span aria-hidden="true">{notice.tone === "danger" ? "×" : notice.tone === "update" ? "↻" : "!"}</span>
          <div>
            <strong>{notice.title}</strong>
            <p>{notice.detail}</p>
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
