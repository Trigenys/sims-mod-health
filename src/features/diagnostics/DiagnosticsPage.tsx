import { useEffect, useMemo, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge, type StatusTone } from "../../components/ui/StatusBadge";
import { diagnosticsGateway, type DiagnosticsGateway } from "./diagnostics.gateway";
import type {
  DiagnosticCandidate,
  DiagnosticReport,
  DiagnosticsSnapshot
} from "./diagnostics.types";

type DiagnosticsPageProps = {
  gateway?: DiagnosticsGateway;
};

export function DiagnosticsPage({
  gateway = diagnosticsGateway
}: DiagnosticsPageProps) {
  const [snapshot, setSnapshot] = useState<DiagnosticsSnapshot | null>(null);
  const [loading, setLoading] = useState(true);

  const analyze = async () => {
    setLoading(true);
    setSnapshot(await gateway.analyze());
    setLoading(false);
  };

  useEffect(() => {
    void analyze();
  }, [gateway]);

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
        Reading local diagnostic reports…
      </section>
    );
  }

  return (
    <>
      <section className="diagnostics-heading" aria-labelledby="diagnostics-title">
        <div>
          <p className="eyebrow">DIAGNOSTIC EVIDENCE</p>
          <h1 id="diagnostics-title">Diagnostics</h1>
          <p className="lede">
            Translate exception reports into evidence-linked candidates without turning correlation into a causality claim.
          </p>
        </div>
        <Button variant="primary" onClick={analyze} disabled={loading}>
          {loading ? "Analyzing…" : "Analyze reports"}
        </Button>
      </section>

      <section className="diagnostics-summary" aria-label="Diagnostics summary">
        <SummaryFact label="Reports" value={snapshot.reports.length} />
        <SummaryFact label="Implicated candidates" value={candidateCount} />
        <SummaryFact
          label="Registry"
          value={snapshot.registryState === "ready" ? "Resolved" : snapshot.registryState}
        />
      </section>

      {snapshot.registryState !== "ready" && (
        <section className="diagnostics-state" role="status">
          <span aria-hidden="true">!</span>
          <div>
            <strong>
              {snapshot.registryState === "offline"
                ? "Registry offline"
                : "Registry resolution is partial"}
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
            <h2>No supported diagnostic report found</h2>
            <p>
              The parser checks recent lastException, lastUIException, MCCC and Better Exceptions reports in the current Sims user folder.
            </p>
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
