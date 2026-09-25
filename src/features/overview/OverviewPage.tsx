import type { CSSProperties } from "react";
import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge } from "../../components/ui/StatusBadge";
import { overviewFixture } from "./overview.fixture";

export function OverviewPage() {
  const data = overviewFixture;

  return (
    <>
      <Topbar
        gameVersion={data.gameVersion}
        platform={data.platform}
        indexedCount={data.indexedCount}
      />

      <section className="page-heading" aria-labelledby="overview-title">
        <div>
          <p className="eyebrow">LIBRARY HEALTH</p>
          <h1 id="overview-title">Your mods are mostly healthy.</h1>
          <p className="lede">
            {data.attentionCount} items need attention before the next long save session.
          </p>
        </div>

        <div className="health-score" aria-label={`Overall health ${data.healthScore} percent`}>
          <div
            className="score-ring"
            aria-hidden="true"
            style={{ "--health-score": `${data.healthScore}%` } as CSSProperties}
          >
            <span>{data.healthScore}</span><small>%</small>
          </div>
          <div>
            <strong>Overall health</strong>
            <span>{data.lastScan}</span>
          </div>
        </div>
      </section>

      <section className="stat-grid" aria-label="Health summary">
        {data.stats.map((stat) => (
          <article className="stat-card" key={stat.label}>
            <StatusBadge tone={stat.tone}>{stat.label}</StatusBadge>
            <strong className="stat-card__value">{stat.value}</strong>
          </article>
        ))}
      </section>

      <section className="content-grid">
        <Panel as="article" className="attention-panel">
          <div className="panel-header">
            <div>
              <span className="section-kicker">Needs attention</span>
              <h2>Resolve the risky stuff first</h2>
            </div>
            <Button variant="text">View all</Button>
          </div>

          <div className="attention-list">
            {data.attention.map((item) => (
              <button className="attention-row" key={item.name}>
                <div className={`item-avatar item-avatar--${item.tone}`} aria-hidden="true">
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
        </Panel>

        <Panel as="aside" className="scan-panel" aria-labelledby="installation-title">
          <span className="section-kicker">Current installation</span>
          <h2 id="installation-title">{data.indexedCount} items indexed</h2>
          <div className="scan-meter" aria-hidden="true"><span /></div>
          <dl className="scan-facts">
            <div><dt>Mods</dt><dd>{data.installation.mods}</dd></div>
            <div><dt>Custom content</dt><dd>{data.installation.customContent}</dd></div>
            <div><dt>Unidentified</dt><dd>{data.installation.unidentified}</dd></div>
            <div><dt>Exact duplicates</dt><dd>{data.installation.exactDuplicates}</dd></div>
          </dl>
          <Button>Open scan details</Button>
        </Panel>
      </section>

      <Panel className="recommendation-strip" aria-labelledby="discover-title">
        <div className="recommendation-icon" aria-hidden="true">✦</div>
        <div>
          <span className="section-kicker">Discover</span>
          <h2 id="discover-title">Your library leans toward relationships, family and realism.</h2>
          <p>
            Recommendations will only show mods compatible with your patch and filtered against known conflicts.
          </p>
        </div>
        <Button>Explore similar mods</Button>
      </Panel>
    </>
  );
}
