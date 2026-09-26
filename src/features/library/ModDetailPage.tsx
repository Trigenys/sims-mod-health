import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge } from "../../components/ui/StatusBadge";
import type { LibraryItem } from "./library.fixture";

type ModDetailPageProps = {
  item: LibraryItem;
  onBack: () => void;
};

const evidenceLabel = {
  fact: "Verified fact",
  inference: "Inferred identification",
  community: "Community report"
} as const;

export function ModDetailPage({ item, onBack }: ModDetailPageProps) {
  const identityLabel =
    item.confidence === "exact"
      ? "Exact fingerprint"
      : item.confidence === "unresolved"
        ? "Unresolved"
        : item.confidence.charAt(0).toUpperCase() + item.confidence.slice(1) + " confidence";

  return (
    <>
      <Topbar
        gameVersion="Patch 1.128.90"
        platform="Windows"
        indexedCount={324}
      />

      <button className="detail-back" onClick={onBack}>
        <span aria-hidden="true">←</span>
        Back to Library
      </button>

      <section className="detail-hero" aria-labelledby="mod-detail-title">
        <div className="detail-hero__identity">
          <span className="detail-monogram" aria-hidden="true">
            {item.canonicalName.slice(0, 2).toUpperCase()}
          </span>
          <div>
            <p className="eyebrow">{item.identified ? "RESOLVED MOD" : "LOCAL FILE"}</p>
            <h1 id="mod-detail-title">{item.canonicalName}</h1>
            <p>
              {item.creator} <span>·</span> {item.category}
            </p>
          </div>
        </div>

        <StatusBadge tone={item.tone}>{item.status}</StatusBadge>
      </section>

      <section className="detail-facts" aria-label="Version and identity facts">
        <Fact label="Installed" value={item.installedVersion ?? "Unknown"} />
        <Fact label="Latest" value={item.latestVersion ?? "Unknown"} />
        <Fact label="Patch" value="1.128.90" />
        <Fact label="Identity" value={identityLabel} />
      </section>

      <div className="detail-actions">
        {item.latestVersion && item.latestVersion !== item.installedVersion && (
          <Button variant="primary">Review update</Button>
        )}
        <Button>{item.enabled ? "Disable" : "Enable"}</Button>
        {item.source !== "Local only" && <Button>Creator page</Button>}
      </div>

      <section className="detail-layout">
        <div className="detail-main">
          <Panel className="detail-section">
            <span className="section-kicker">What it does</span>
            <h2>Purpose and scope</h2>
            <p className="detail-description">{item.whatItDoes}</p>
          </Panel>

          <Panel className="detail-section">
            <div className="panel-header">
              <div>
                <span className="section-kicker">Evidence</span>
                <h2>Why the app says this</h2>
              </div>
              <span className="evidence-count">{item.evidence.length} signals</span>
            </div>

            <div className="evidence-list">
              {item.evidence.map((evidence) => (
                <article
                  className={"evidence-card evidence-card--" + evidence.type}
                  key={evidence.type + "-" + evidence.title}
                >
                  <div className="evidence-card__label">
                    <span aria-hidden="true">
                      {evidence.type === "fact" ? "✓" : evidence.type === "inference" ? "≈" : "◌"}
                    </span>
                    {evidenceLabel[evidence.type]}
                  </div>
                  <strong>{evidence.title}</strong>
                  <p>{evidence.detail}</p>
                  {evidence.source && <small>Source: {evidence.source}</small>}
                </article>
              ))}
            </div>
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">Local files</span>
            <h2>Installed artifacts</h2>
            <ul className="file-list">
              {item.localFiles.map((file) => (
                <li key={file}>
                  <span aria-hidden="true">⌘</span>
                  <code>{file}</code>
                  <span>{item.enabled ? "Enabled" : "Disabled"}</span>
                </li>
              ))}
            </ul>
          </Panel>
        </div>

        <aside className="detail-side">
          <Panel className="detail-section">
            <span className="section-kicker">Dependencies</span>
            <h2>
              {item.dependencies.length === 0
                ? "No required dependencies"
                : item.dependencies.length + " required"}
            </h2>
            {item.dependencies.length > 0 && (
              <ul className="dependency-list">
                {item.dependencies.map((dependency) => (
                  <li key={dependency.name}>
                    <span>
                      <strong>{dependency.name}</strong>
                      <small>{dependency.state}</small>
                    </span>
                    <span aria-hidden="true">›</span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">Source</span>
            <h2>{item.source}</h2>
            <dl className="detail-source">
              <div><dt>Creator</dt><dd>{item.creator}</dd></div>
              <div><dt>Category</dt><dd>{item.category}</dd></div>
              <div><dt>File type</dt><dd>{item.kind}</dd></div>
              <div><dt>State</dt><dd>{item.enabled ? "Enabled" : "Disabled"}</dd></div>
            </dl>
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">Related mods</span>
            <h2>Nearby in your library</h2>
            {item.relatedMods.length === 0 ? (
              <p className="detail-muted">No related canonical mods yet.</p>
            ) : (
              <div className="related-list">
                {item.relatedMods.map((name) => (
                  <button key={name}>
                    {name}
                    <span aria-hidden="true">›</span>
                  </button>
                ))}
              </div>
            )}
          </Panel>
        </aside>
      </section>
    </>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
