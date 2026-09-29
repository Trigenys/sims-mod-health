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
import { gameContentMeasurement, type GameContentMeasurementReason } from "../game-content/gameContent.presenter";
import type { GameContentHealthSnapshot } from "../game-content/gameContent.types";
import { overviewGateway, type OverviewGateway } from "./overview.gateway";
import type { OverviewSnapshot, ScanProgress } from "./overview.types";
import {
  localizeAttentionBadge,
  localizeAttentionCreator,
  localizeOverviewDetail,
  localizeOverviewExplanation
} from "./overview.i18n";
import { useI18n } from "../../i18n/i18n";
import { SimsSetupPanel } from "../setup/SimsSetupPanel";

type OverviewPageProps = {
  gateway?: OverviewGateway;
  contentGateway?: GameContentGateway;
  onOpenSettings?: () => void;
};

export function OverviewPage({
  gateway = overviewGateway,
  contentGateway = gameContentGateway,
  onOpenSettings
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

  const measurement = gameContentMeasurement(gameContent, data.gameVersion);
  const overallHealthScore = measurement.compatibilityReady ? data.healthScore : null;
  const scoreExplanation = measurement.compatibilityReady
    ? localizeOverviewExplanation(data.healthScoreExplanation, t)
    : measurementExplanation(measurement.reason, t);
  const headline = tx(overviewHeadline(data, overallHealthScore));
  const unifiedAttention = data.attentionCount + (measurement.attentionCount ?? 0);
  const attentionCopy =
    unifiedAttention === 0
      ? t("Nothing needs your attention right now.")
      : t("{{count}} items need your attention.", { count: unifiedAttention });

  return (
    <>
      <Topbar
        gameVersion={data.gameVersion ? t("Patch {{version}}", { version: data.gameVersion }) : t("Game version not detected yet")}
        platform={formatPlatform(data.platform)}
        indexedCount={data.indexedCount}
        onScan={data.hasInstallation ? runScan : undefined}
        scanning={scanning}
        scanDisabled={!data.hasInstallation}
      />

      <OverviewStateBanner data={data} scanning={scanning} scanError={scanError} />

      {!data.hasInstallation ? (
        <SimsSetupPanel onScanComplete={refresh} />
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
                overallHealthScore === null
                  ? t("Overall health unavailable")
                  : t("Overall health {{score}} percent", { score: overallHealthScore })
              }
              title={scoreExplanation}
            >
              <div
                className={"score-ring" + (overallHealthScore === null ? " score-ring--unknown" : "")}
                aria-hidden="true"
                style={
                  {
                    "--health-score": overallHealthScore === null ? "0%" : overallHealthScore + "%"
                  } as CSSProperties
                }
              >
                <span>{overallHealthScore ?? "—"}</span>
                {overallHealthScore !== null && <small>%</small>}
              </div>
              <div>
                <strong>{t("Setup health")}</strong>
                <span>{tx(scanLabel(data))}</span>
              </div>
            </div>
          </section>

          <details className="health-explanation">
            <summary>{t("How this score works")}</summary>
            <p>{scoreExplanation}</p>
          </details>

          {!measurement.compatibilityReady && (
            <MeasurementNotice
              reason={measurement.reason}
              onOpenSettings={onOpenSettings}
            />
          )}

          <GameInstallationSummary
            health={gameContent}
            modCount={data.indexedCount}
            fallbackVersion={data.gameVersion}
          />

          <section className="stat-grid" aria-label={t("Mod health summary")}>
            <Stat
              label={t("Healthy")}
              value={data.registryState === "ready" ? data.healthCounts.healthy : t("Not checked")}
              tone="healthy"
              unavailable={data.registryState !== "ready"}
            />
            <Stat
              label={t("Updates")}
              value={data.registryState === "ready" ? data.healthCounts.updates : t("Not checked")}
              tone="update"
              unavailable={data.registryState !== "ready"}
            />
            <Stat
              label={t("Conflicts")}
              value={
                data.healthCounts.conflicts > 0
                  ? data.healthCounts.conflicts
                  : data.registryState === "ready"
                    ? 0
                    : t("Not checked")
              }
              tone="warning"
              unavailable={data.healthCounts.conflicts === 0 && data.registryState !== "ready"}
            />
            <Stat
              label={t("Not identified yet")}
              value={data.installation.unidentified ?? t("Not checked")}
              tone="muted"
              unavailable={data.installation.unidentified === null}
            />
          </section>

          <section className="content-grid">
            <Panel as="article" className="attention-panel">
              <div className="panel-header">
                <div>
                  <span className="section-kicker">{t("Needs attention")}</span>
                  <h2>{t("Start with the most important items")}</h2>
                </div>
                <Button variant="text">{t("View all")}</Button>
              </div>

              {data.attention.length === 0 ? (
                <div className="attention-empty">
                  <span aria-hidden="true">✓</span>
                  <div>
                    <strong>{t("Nothing urgent found")}</strong>
                    <p>{t("We did not find anything that needs immediate action.")}</p>
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
                        <span>{localizeAttentionCreator(item.creator, t)} · {localizeOverviewDetail(item.detail, t)}</span>
                      </div>
                      <StatusBadge tone={item.tone}>{localizeAttentionBadge(item.badge, t)}</StatusBadge>
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
              <h2 id="discover-title">{t("Find mods that fit your current setup.")}</h2>
              <p>
                {t("Recommendations appear only when we have enough information about your game and installed mods.")}
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
        title: "Scan stopped before finishing",
        detail: "Try again. If it keeps failing, check that your Sims 4 folders are still available."
      });
    }

    if (scanning || data.scan.status === "running") {
      result.push({
        tone: "update",
        title: "Scanning your mods",
        detail: "We are checking your local files now. Compatibility and update checks refresh when the scan finishes."
      });
    } else if (data.scan.stale) {
      result.push({
        tone: "muted",
        title: "Your scan is out of date",
        detail: "Run Scan again to refresh the results before relying on them."
      });
    } else if (data.scan.partial) {
      result.push({
        tone: "warning",
        title: "Some files could not be checked",
        detail: "Your current results are still available, but scanning again may fill in the missing details."
      });
    }

    if (data.registryState === "offline") {
      result.push({
        tone: "muted",
        title: "Online checks are temporarily unavailable",
        detail: "Your local scan still works. Mod names, compatibility and update information may be incomplete until online checks are available again."
      });
    } else if (data.registryState === "partial") {
      result.push({
        tone: "warning",
        title: "Some online checks are unavailable",
        detail: "Your local scan is available, but some compatibility and update details could not be loaded."
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
            <strong>{localizeOverviewDetail(notice.title, t)}</strong>
            <p>{localizeOverviewDetail(notice.detail, t)}</p>
          </div>
        </div>
      ))}
    </div>
  );
}

function Stat({
  label,
  value,
  tone,
  unavailable = false
}: {
  label: string;
  value: number | string;
  tone: "healthy" | "update" | "warning" | "muted";
  unavailable?: boolean;
}) {
  return (
    <article className={unavailable ? "stat-card stat-card--unavailable" : "stat-card"}>
      <StatusBadge tone={tone}>{label}</StatusBadge>
      <strong className="stat-card__value">{value}</strong>
    </article>
  );
}

function MeasurementNotice({
  reason,
  onOpenSettings
}: {
  reason: GameContentMeasurementReason | null;
  onOpenSettings?: () => void;
}) {
  const { t } = useI18n();
  const localIssue = reason === "game_missing" || reason === "version_missing";

  return (
    <Panel className="measurement-notice" as="section">
      <div className="measurement-notice__icon" aria-hidden="true">!</div>
      <div>
        <span className="section-kicker">{t("SCORE PAUSED")}</span>
        <h2>{measurementTitle(reason, t)}</h2>
        <p>{measurementExplanation(reason, t)}</p>
      </div>
      {localIssue && onOpenSettings && (
        <Button variant="secondary" onClick={onOpenSettings}>
          {t("Review game folders")}
        </Button>
      )}
    </Panel>
  );
}

function measurementTitle(
  reason: GameContentMeasurementReason | null,
  t: ReturnType<typeof useI18n>["t"]
) {
  if (reason === "game_missing") return t("We still need your game installation");
  if (reason === "version_missing") return t("We still need your game version");
  if (reason === "manifest_missing") return t("Compatibility checks are unavailable right now");
  if (reason === "manifest_stale") return t("Compatibility information may be out of date");
  return t("Game compatibility could not be confirmed");
}

function measurementExplanation(
  reason: GameContentMeasurementReason | null,
  t: ReturnType<typeof useI18n>["t"]
) {
  if (reason === "game_missing") {
    return t("Your Mods scan is available, but the game installation has not been confirmed. We will not turn missing game data into zeroes or a health score.");
  }
  if (reason === "version_missing") {
    return t("We found the game, but could not read its version. Your local Mods facts remain visible, but the overall score stays hidden until the version is known.");
  }
  if (reason === "manifest_missing") {
    return t("Your game and local files are available, but online compatibility data is not. Local facts remain visible; the overall score and game attention stay unmeasured.");
  }
  if (reason === "manifest_stale") {
    return t("We have cached compatibility information, but it may be old. We keep the local facts visible and pause the overall score until fresh checks are available.");
  }
  return t("We do not have enough reliable game compatibility information to calculate an overall score. Local scan facts remain available.");
}

function overviewHeadline(data: OverviewSnapshot, overallHealthScore: number | null) {
  if (data.registryState === "offline") {
    return "Your local scan is ready. Some online checks are temporarily unavailable.";
  }
  if (overallHealthScore === null) {
    return "We need a little more information before rating your setup.";
  }
  if (overallHealthScore >= 90) {
    return "Your setup looks good based on the checks we could complete.";
  }
  if (overallHealthScore >= 70) {
    return "A few things are worth checking.";
  }
  return "Start with the items that need attention.";
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
