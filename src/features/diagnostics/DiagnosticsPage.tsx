import { useEffect, useMemo, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge, type StatusTone } from "../../components/ui/StatusBadge";
import { useI18n } from "../../i18n/I18nProvider";
import { diagnosticsGateway, type DiagnosticsGateway } from "./diagnostics.gateway";
import type {
  DiagnosticCandidate,
  DiagnosticReport,
  DiagnosticsSnapshot
} from "./diagnostics.types";

type DiagnosticsPageProps = {
  gateway?: DiagnosticsGateway;
  embedded?: boolean;
};

export function DiagnosticsPage({
  gateway = diagnosticsGateway,
  embedded = false
}: DiagnosticsPageProps) {
  const { t } = useI18n();
  const [snapshot, setSnapshot] = useState<DiagnosticsSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [telemetryEnabled, setTelemetryEnabled] = useState(false);
  const [privacyBusy, setPrivacyBusy] = useState(true);
  const [privacyError, setPrivacyError] = useState<string | null>(null);

  const analyze = async () => {
    setLoading(true);
    setSnapshot(await gateway.analyze());
    setLoading(false);
  };

  useEffect(() => {
    void analyze();
    gateway
      .getPrivacyPreferences()
      .then((preferences) => {
        setTelemetryEnabled(preferences.diagnosticTelemetryEnabled);
        setPrivacyError(null);
      })
      .catch(() => {
        setTelemetryEnabled(false);
        setPrivacyError("Privacy settings could not be loaded. Diagnostic telemetry remains off.");
      })
      .finally(() => setPrivacyBusy(false));
  }, [gateway]);

  const updateTelemetryConsent = async (enabled: boolean) => {
    setPrivacyBusy(true);
    setPrivacyError(null);

    try {
      const preferences = await gateway.setDiagnosticTelemetryConsent(enabled);
      setTelemetryEnabled(preferences.diagnosticTelemetryEnabled);
    } catch {
      setTelemetryEnabled(false);
      setPrivacyError("Consent could not be saved. Diagnostic telemetry remains off.");
    } finally {
      setPrivacyBusy(false);
    }
  };

  const candidateCount = useMemo(
    () =>
      snapshot?.reports.reduce(
        (total, report) => total + report.candidates.length,
        0
      ) ?? 0,
    [snapshot]
  );

  if (!snapshot) {
    return (
      <section className="diagnostics-loading" role="status">
        {t("diagnostics.loading")}
      </section>
    );
  }

  return (
    <>
      {!embedded && (
        <section className="diagnostics-heading" aria-labelledby="diagnostics-title">
          <div>
            <p className="eyebrow">{t("diagnostics.eyebrow")}</p>
            <h1 id="diagnostics-title">{t("diagnostics.title")}</h1>
            <p className="lede">{t("diagnostics.lede")}</p>
          </div>
          <Button variant="primary" onClick={analyze} disabled={loading}>
            {loading ? t("diagnostics.analyzing") : t("diagnostics.analyze")}
          </Button>
        </section>
      )}

      {embedded && (
        <div className="health-embedded-actions">
          <div>
            <span className="section-kicker">{t("diagnostics.eyebrow")}</span>
            <strong>{t("diagnostics.embedded")}</strong>
          </div>
          <Button variant="secondary" onClick={analyze} disabled={loading}>
            {loading ? "Analyzing…" : "Analyze reports"}
          </Button>
        </div>
      )}

      <section className="diagnostics-summary" aria-label={t("diagnostics.summary")}>
        <SummaryFact label={t("diagnostics.reports")} value={snapshot.reports.length} />
        <SummaryFact label={t("diagnostics.candidates")} value={candidateCount} />
        <SummaryFact
          label={t("diagnostics.registry")}
          value={snapshot.registryState === "ready" ? t("diagnostics.resolved") : snapshot.registryState}
        />
      </section>

      <section className="diagnostics-privacy" aria-label={t("diagnostics.privacy")}>
        <div>
          <span className="section-kicker">PRIVACY</span>
          <strong>{t("diagnostics.telemetry", { state: telemetryEnabled ? t("settings.on") : t("settings.off") })}</strong>
          <p>{t("diagnostics.telemetryCopy")}</p>
          {privacyError && <small role="status">{privacyError}</small>}
        </div>
        <label className="privacy-toggle">
          <input
            type="checkbox"
            checked={telemetryEnabled}
            disabled={privacyBusy}
            onChange={(event) => void updateTelemetryConsent(event.target.checked)}
          />
          <span>{t("diagnostics.allowTelemetry")}</span>
        </label>
      </section>

      {snapshot.registryState !== "ready" && (
        <section className="diagnostics-state" role="status">
          <span aria-hidden="true">!</span>
          <div>
            <strong>
              {snapshot.registryState === "offline"
                ? t("diagnostics.offline")
                : t("diagnostics.partial")}
            </strong>
            <p>
              Local module, filename and resource evidence remains available.{" "}
              {snapshot.registryDetail}
            </p>
          </div>
        </section>
      )}

      {snapshot.reports.length === 0 ? (
        <Panel className="diagnostics-empty">
          <div className="diagnostics-empty__icon" aria-hidden="true">⌁</div>
          <div>
            <h2>{t("diagnostics.noReport")}</h2>
            <p>{t("diagnostics.noReportCopy")}</p>
          </div>
        </Panel>
      ) : (
        <section className="diagnostics-list" aria-label="Parsed diagnostic reports">
          {snapshot.reports.map((report) => (
            <DiagnosticReportCard
              key={
                report.diagnosticId ??
                report.sourceKind + "-" + report.reportName + "-" + report.observedAt
              }
              report={report}
            />
          ))}
        </section>
      )}
    </>
  );
}

function DiagnosticReportCard({ report }: { report: DiagnosticReport }) {
  const tone = parseTone(report.parseStatus);

  return (
    <Panel as="article" className="diagnostic-report">
      <div className="diagnostic-report__header">
        <div>
          <span className="section-kicker">{sourceLabel(report.sourceKind)}</span>
          <h2>{report.reportName}</h2>
        </div>
        <StatusBadge tone={tone}>{statusLabel(report.parseStatus)}</StatusBadge>
      </div>

      <p className="diagnostic-note">{report.note}</p>

      {report.telemetryPreview.redactionsApplied > 0 && (
        <div className="privacy-chip">
          <span aria-hidden="true">✓</span>
          {report.telemetryPreview.redactionsApplied} personal/path values redacted from telemetry preview
        </div>
      )}

      {report.candidates.length > 0 ? (
        <div className="diagnostic-candidates">
          {report.candidates.map((candidate) => (
            <CandidateCard
              candidate={candidate}
              key={candidate.localFileId}
            />
          ))}
        </div>
      ) : (
        <div className="diagnostic-no-candidate">
          <strong>No installed mod candidate linked</strong>
          <span>
            The report remains stored as normalized evidence even when no local artifact matches.
          </span>
        </div>
      )}

      {report.observations.length > 0 && (
        <details className="diagnostic-observations">
          <summary>{report.observations.length} normalized observations</summary>
          <ul>
            {report.observations.map((observation, index) => (
              <li key={observation.kind + "-" + observation.value + "-" + index}>
                <span>{observation.kind}</span>
                <code>{observation.value}</code>
                <small>{observation.context}</small>
              </li>
            ))}
          </ul>
        </details>
      )}
    </Panel>
  );
}

function CandidateCard({ candidate }: { candidate: DiagnosticCandidate }) {
  return (
    <article className="diagnostic-candidate">
      <div className="diagnostic-candidate__top">
        <div>
          <span className="section-kicker">IMPLICATED CANDIDATE</span>
          <strong>{candidate.relativePath}</strong>
          <small>
            Correlated with report evidence · {candidate.confidenceScore}% local match confidence
          </small>
        </div>
        <span className={"diagnostic-confidence diagnostic-confidence--" + candidate.confidence}>
          {candidate.confidence}
        </span>
      </div>

      <div className="diagnostic-evidence">
        {candidate.evidence.map((evidence) => (
          <div key={evidence.kind + "-" + evidence.reference}>
            <span>{evidence.kind}</span>
            <code>{evidence.reference}</code>
            <p>{evidence.explanation}</p>
          </div>
        ))}
      </div>

      {candidate.canonical && (
        <div className="canonical-match">
          <span aria-hidden="true">✓</span>
          <div>
            <strong>
              {candidate.canonical.deterministic
                ? "Canonical artifact resolved"
                : "Canonical candidate resolved"}
            </strong>
            <small>
              Registry confidence {candidate.canonical.confidence}
            </small>
          </div>
        </div>
      )}
    </article>
  );
}

function SummaryFact({
  label,
  value
}: {
  label: string;
  value: string | number;
}) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function parseTone(status: string): StatusTone {
  if (status === "parsed") return "healthy";
  if (status === "error") return "danger";
  if (status === "malformed") return "warning";
  return "muted";
}

function statusLabel(status: string) {
  if (status === "parsed") return "Parsed";
  if (status === "malformed") return "Malformed";
  if (status === "unsupported") return "Unsupported";
  if (status === "error") return "Read error";
  return status;
}

function sourceLabel(source: string) {
  if (source === "last_exception") return "LAST EXCEPTION";
  if (source === "last_ui_exception") return "LAST UI EXCEPTION";
  if (source === "mccc") return "MCCC REPORT";
  if (source === "better_exceptions") return "BETTER EXCEPTIONS";
  return source.replaceAll("_", " ").toUpperCase();
}
