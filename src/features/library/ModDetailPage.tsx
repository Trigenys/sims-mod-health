import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge } from "../../components/ui/StatusBadge";
import type { LibraryItem } from "./library.fixture";
import { useI18n } from "../../i18n/i18n";
import { localizeLibraryGeneratedCopy } from "./library.i18n";

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
  const { t, tx } = useI18n();
  const identityLabel =
    item.confidence === "exact"
      ? t("Exact fingerprint")
      : item.confidence === "unresolved"
        ? t("Unresolved")
        : tx(item.confidence.charAt(0).toUpperCase() + item.confidence.slice(1) + " confidence");

  return (
    <>
      <Topbar
        gameVersion={item.gameVersion ? t("Patch {{version}}", { version: item.gameVersion }) : t("Patch unknown")}
        platform="Windows"
        indexedCount={item.libraryCount ?? 0}
      />

      <button className="detail-back" onClick={onBack}>
        <span aria-hidden="true">←</span>
        {t("Back to Library")}
      </button>

      <section className="detail-hero" aria-labelledby="mod-detail-title">
        <div className="detail-hero__identity">
          <span className="detail-monogram" aria-hidden="true">
            {item.canonicalName.slice(0, 2).toUpperCase()}
          </span>
          <div>
            <p className="eyebrow">{item.identified ? t("RESOLVED MOD") : t("LOCAL FILE")}</p>
            <h1 id="mod-detail-title">{item.canonicalName}</h1>
            <p>
              {tx(item.creator)} <span>·</span> {tx(item.category)}
            </p>
          </div>
        </div>

        <StatusBadge tone={item.tone}>{tx(item.status)}</StatusBadge>
      </section>

      <section className="detail-facts" aria-label={t("Version and identity facts")}>
        <Fact label={t("Installed")} value={item.installedVersion ?? t("Unknown")} />
        <Fact label={t("Latest")} value={item.latestVersion ?? t("Unknown")} />
        <Fact label={t("Patch")} value={item.gameVersion ?? t("Unknown")} />
        <Fact label={t("Identity")} value={identityLabel} />
      </section>

      <div className="detail-actions">
        {item.latestVersion && item.latestVersion !== item.installedVersion && (
          <Button variant="primary">{t("Review update")}</Button>
        )}
        {item.source !== "Local only" && <Button>{item.enabled ? t("Disable") : t("Enable")}</Button>}
        {item.source !== "Local only" && <Button>{t("Creator page")}</Button>}
      </div>

      <section className="detail-layout">
        <div className="detail-main">
          <Panel className="detail-section">
            <span className="section-kicker">{t("What it does")}</span>
            <h2>{t("Purpose and scope")}</h2>
            <p className="detail-description">{localizeLibraryGeneratedCopy(item.whatItDoes, t)}</p>
          </Panel>

          <Panel className="detail-section">
            <div className="panel-header">
              <div>
                <span className="section-kicker">{t("Evidence")}</span>
                <h2>{t("Why the app says this")}</h2>
              </div>
              <span className="evidence-count">{t("{{count}} signals", { count: item.evidence.length })}</span>
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
                    {tx(evidenceLabel[evidence.type])}
                  </div>
                  <strong>{tx(evidence.title)}</strong>
                  <p>{localizeLibraryGeneratedCopy(evidence.detail, t)}</p>
                  {evidence.source && <small>{t("Source: {{source}}", { source: evidence.source })}</small>}
                </article>
              ))}
            </div>
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">{t("Local files")}</span>
            <h2>{t("Installed artifacts")}</h2>
            <ul className="file-list">
              {item.localFiles.map((file) => (
                <li key={file}>
                  <span aria-hidden="true">⌘</span>
                  <code>{file}</code>
                  <span>{item.enabled ? t("Enabled") : t("Disabled")}</span>
                </li>
              ))}
            </ul>
          </Panel>
        </div>

        <aside className="detail-side">
          <Panel className="detail-section">
            <span className="section-kicker">{t("Dependencies")}</span>
            <h2>
              {item.dependencies.length === 0
                ? t("No required dependencies")
                : t("{{count}} required", { count: item.dependencies.length })}
            </h2>
            {item.dependencies.length > 0 && (
              <ul className="dependency-list">
                {item.dependencies.map((dependency) => (
                  <li key={dependency.name}>
                    <span>
                      <strong>{dependency.name}</strong>
                      <small>{tx(dependency.state)}</small>
                    </span>
                    <span aria-hidden="true">›</span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">{t("Source")}</span>
            <h2>{tx(item.source)}</h2>
            <dl className="detail-source">
              <div><dt>{t("Creator")}</dt><dd>{tx(item.creator)}</dd></div>
              <div><dt>{t("Category")}</dt><dd>{tx(item.category)}</dd></div>
              <div><dt>{t("File type")}</dt><dd>{tx(item.kind)}</dd></div>
              <div><dt>{t("State")}</dt><dd>{item.enabled ? t("Enabled") : t("Disabled")}</dd></div>
            </dl>
          </Panel>

          <Panel className="detail-section">
            <span className="section-kicker">{t("Related mods")}</span>
            <h2>{t("Nearby in your library")}</h2>
            {item.relatedMods.length === 0 ? (
              <p className="detail-muted">{t("No related canonical mods yet.")}</p>
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
