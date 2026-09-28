import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Topbar } from "../../components/layout/Topbar";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { diagnosticsGateway } from "../diagnostics/diagnostics.gateway";
import { gameContentGateway } from "../game-content/gameContent.gateway";
import { formatProvider } from "../game-content/gameContent.presenter";
import type {
  GameContentSnapshot,
  ProviderUpdateCapability
} from "../game-content/gameContent.types";
import { useI18n } from "../../i18n/i18n";

type SettingsSection =
  | "paths"
  | "privacy"
  | "scan"
  | "recovery"
  | "appearance";

type InstallationCandidate = {
  root: string;
  modsRoot: string;
  source: "knownDocuments" | "oneDriveFallback" | "manual" | string;
  modsAvailable: boolean;
  version:
    | { status: "available"; version: { normalized: string } }
    | { status: "missing" }
    | { status: "invalid"; reason: string };
};

const sections: { value: SettingsSection; label: string; icon: string }[] = [
  { value: "paths", label: "Paths & Registry", icon: "⌂" },
  { value: "privacy", label: "Checks & Privacy", icon: "✓" },
  { value: "scan", label: "Scan Behavior", icon: "↻" },
  { value: "recovery", label: "Recovery & Data", icon: "◫" },
  { value: "appearance", label: "Appearance", icon: "◐" }
];

export function SettingsPage() {
  const { t, tx } = useI18n();
  const [section, setSection] = useState<SettingsSection>("paths");
  const [installations, setInstallations] = useState<InstallationCandidate[]>([]);
  const [detecting, setDetecting] = useState(true);
  const [gameInventory, setGameInventory] = useState<GameContentSnapshot>({ installations: [] });
  const [providerCapability, setProviderCapability] = useState<ProviderUpdateCapability | null>(null);
  const [telemetryEnabled, setTelemetryEnabled] = useState(false);
  const [privacyBusy, setPrivacyBusy] = useState(true);
  const [privacyError, setPrivacyError] = useState<string | null>(null);

  const detect = async () => {
    setDetecting(true);
    try {
      const [values, content, capability] = await Promise.all([
        invoke<InstallationCandidate[]>("discover_sims_installations").catch(() => []),
        gameContentGateway.refreshInventory(),
        gameContentGateway.loadCapability()
      ]);
      setInstallations(values);
      setGameInventory(content);
      setProviderCapability(capability);
    } finally {
      setDetecting(false);
    }
  };

  useEffect(() => {
    void detect();

    diagnosticsGateway
      .getPrivacyPreferences()
      .then((preferences) => {
        setTelemetryEnabled(preferences.diagnosticTelemetryEnabled);
        setPrivacyError(null);
      })
      .catch(() => {
        setTelemetryEnabled(false);
        setPrivacyError("Privacy settings could not be loaded. Telemetry remains off.");
      })
      .finally(() => setPrivacyBusy(false));
  }, []);

  const activeInstallation = useMemo(
    () => installations.find((item) => item.modsAvailable) ?? installations[0],
    [installations]
  );

  const setTelemetry = async (enabled: boolean) => {
    setPrivacyBusy(true);
    setPrivacyError(null);
    try {
      const preferences =
        await diagnosticsGateway.setDiagnosticTelemetryConsent(enabled);
      setTelemetryEnabled(preferences.diagnosticTelemetryEnabled);
    } catch {
      setTelemetryEnabled(false);
      setPrivacyError("Consent could not be saved. Telemetry remains off.");
    } finally {
      setPrivacyBusy(false);
    }
  };

  return (
    <>
      <Topbar
        gameVersion={versionLabel(activeInstallation, t)}
        platform="Windows"
        indexedCount={0}
      />

      <section className="settings-hero" aria-labelledby="settings-title">
        <div>
          <p className="eyebrow">{t("PREFERENCES & ENGINE CONFIGURATION")}</p>
          <h1 id="settings-title">{t("System Settings")}</h1>
          <p className="lede">
            {t("Configure installation paths, privacy boundaries, scanning behavior and recovery storage.")}
          </p>
        </div>
        <div className="settings-engine-state">
          <span className="status-dot" aria-hidden="true" />
          <div>
            <span>{t("Engine mode")}</span>
            <strong>{t("Local-first")}</strong>
          </div>
        </div>
      </section>

      <div className="settings-layout">
        <nav className="settings-nav" aria-label={t("Settings sections")}>
          {sections.map((item) => (
            <button
              type="button"
              key={item.value}
              aria-current={section === item.value ? "page" : undefined}
              className={
                section === item.value
                  ? "settings-nav__item settings-nav__item--active"
                  : "settings-nav__item"
              }
              onClick={() => setSection(item.value)}
            >
              <span aria-hidden="true">{item.icon}</span>
              {t(item.label as "Paths & Registry" | "Checks & Privacy" | "Scan Behavior" | "Recovery & Data" | "Appearance")}
            </button>
          ))}

          <div className="settings-nav__note">
            <span className="section-kicker">{t("BETA POLICY")}</span>
            <p>
              {t("Local evidence remains usable when Registry-backed features are unavailable.")}
            </p>
          </div>
        </nav>

        <section className="settings-content">
          {section === "paths" && (
            <PathsSettings
              active={activeInstallation}
              installations={installations}
              detecting={detecting}
              onDetect={detect}
              gameInventory={gameInventory}
              providerCapability={providerCapability}
            />
          )}

          {section === "privacy" && (
            <PrivacySettings
              enabled={telemetryEnabled}
              busy={privacyBusy}
              error={privacyError ? tx(privacyError) : null}
              onChange={setTelemetry}
            />
          )}

          {section === "scan" && <ScanSettings />}
          {section === "recovery" && <RecoverySettings />}
          {section === "appearance" && <AppearanceSettings />}
        </section>
      </div>
    </>
  );
}

function PathsSettings({
  active,
  installations,
  detecting,
  onDetect,
  gameInventory,
  providerCapability
}: {
  active?: InstallationCandidate;
  installations: InstallationCandidate[];
  detecting: boolean;
  onDetect: () => Promise<void>;
  gameInventory: GameContentSnapshot;
  providerCapability: ProviderUpdateCapability | null;
}) {
  const programInstallation = gameInventory.installations[0];

  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">LOCAL INSTALLATION</span>
          <h2>Sims 4 & Mods paths</h2>
          <p>
            Paths are detected locally. The scanner reads the selected Mods directory without uploading raw files.
          </p>
        </div>
        <Button variant="secondary" onClick={() => void onDetect()} disabled={detecting}>
          {detecting ? "Detecting…" : "Rescan locations"}
        </Button>
      </div>

      <Panel className="settings-card settings-card--featured">
        <div className="settings-card__header">
          <div>
            <span className="section-kicker">ACTIVE SIMS 4 ROOT</span>
            <h3>{active ? sourceLabel(active.source) : "No installation detected"}</h3>
          </div>
          <span className={active?.modsAvailable ? "settings-ok" : "settings-muted"}>
            {active?.modsAvailable ? "Mods folder available" : "Not available"}
          </span>
        </div>

        <PathRow
          label="Sims 4 user folder"
          value={active?.root ?? "Run detection in the desktop application"}
        />
        <PathRow
          label="Mods folder"
          value={active?.modsRoot ?? "No Mods directory detected"}
        />
        <PathRow
          label="Game patch"
          value={
            programInstallation?.build.version?.normalized
              ? "Patch " + programInstallation.build.version.normalized
              : versionLabel(active)
          }
        />
        <PathRow
          label="Game program folder"
          value={programInstallation?.installRoot ?? "No program installation detected"}
        />
        <PathRow
          label="Update provider"
          value={formatProvider(programInstallation?.provider ?? providerCapability?.provider ?? "unknown")}
        />
        <PathRow
          label="Installed packs"
          value={String(programInstallation?.packs.length ?? 0)}
        />
      </Panel>

      {installations.length > 1 && (
        <Panel className="settings-card">
          <span className="section-kicker">OTHER DETECTED ROOTS</span>
          <div className="settings-installations">
            {installations.slice(1).map((item) => (
              <div key={item.root}>
                <strong>{sourceLabel(item.source)}</strong>
                <code>{item.root}</code>
              </div>
            ))}
          </div>
        </Panel>
      )}

      <Panel className="settings-card">
        <span className="section-kicker">UPDATE PROVIDER</span>
        <h3>Official provider handoff only</h3>
        <p>
          {providerCapability?.detail
            ?? "Provider capability is resolved locally when a game installation is available."}
          {" "}After an EA app or Steam update, Sims Mod Health rescans local game and pack evidence before declaring success.
        </p>
      </Panel>

      <Panel className="settings-card">
        <span className="section-kicker">REGISTRY BEHAVIOR</span>
        <h3>Local scan remains the baseline</h3>
        <p>
          Registry identity, compatibility, relationships and Discover enrich local evidence.
          If the Registry is unreachable, the app keeps local inventory, duplicate and diagnostic evidence visible.
        </p>
      </Panel>
    </>
  );
}

function PrivacySettings({
  enabled,
  busy,
  error,
  onChange
}: {
  enabled: boolean;
  busy: boolean;
  error: string | null;
  onChange: (enabled: boolean) => Promise<void>;
}) {
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">PRIVACY</span>
          <h2>Checks & privacy</h2>
          <p>
            Diagnostic telemetry is explicit opt-in. Raw diagnostic reports and raw mod files are not uploaded automatically.
          </p>
        </div>
      </div>

      <Panel className="settings-card settings-toggle-card">
        <div>
          <h3>Redacted diagnostic telemetry</h3>
          <p>
            When enabled, only redacted summary metadata is eligible for telemetry. Invalid consent state fails closed.
          </p>
          {error && <small className="settings-error">{error}</small>}
        </div>
        <label className="settings-switch">
          <input
            type="checkbox"
            checked={enabled}
            disabled={busy}
            onChange={(event) => void onChange(event.target.checked)}
          />
          <span>{enabled ? "On" : "Off"}</span>
        </label>
      </Panel>

      <div className="settings-grid">
        <PolicyCard
          title="Local parsing"
          detail="DBPF, TS4Script and supported diagnostic reports are parsed on-device with bounded readers."
        />
        <PolicyCard
          title="Registry requests"
          detail="Canonical identity uses constrained artifact metadata and supported fingerprints rather than raw file uploads."
        />
        <PolicyCard
          title="Evidence language"
          detail="Correlation and potential conflicts remain distinct from deterministic breakage."
        />
      </div>
    </>
  );
}

function ScanSettings() {
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">SCANNER</span>
          <h2>Scan behavior</h2>
          <p>
            The production scanner defaults to incremental work so unchanged files can be skipped safely.
          </p>
        </div>
      </div>
      <div className="settings-grid">
        <PolicyCard
          title="Incremental by default"
          detail="Known unchanged artifacts reuse persisted evidence; changed files are hashed and re-inspected."
        />
        <PolicyCard
          title="Bounded inspection"
          detail="Archive, package, script and diagnostic parsing use explicit limits rather than unbounded recursive work."
        />
        <PolicyCard
          title="No automatic destructive cleanup"
          detail="Health findings are reviewable evidence. Unknown files are not silently removed."
        />
      </div>
    </>
  );
}

function RecoverySettings() {
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">RECOVERY & STORAGE</span>
          <h2>Safe mutation policy</h2>
          <p>
            Supported updates use app-controlled staging and verified restore points.
          </p>
        </div>
      </div>
      <div className="settings-grid">
        <PolicyCard
          title="Restore points"
          detail="The original artifact is verified before the Mods folder is mutated."
        />
        <PolicyCard
          title="Persistent journal"
          detail="Interrupted update transactions remain recoverable after restart."
        />
        <PolicyCard
          title="Rollback guard"
          detail="Rollback refuses to overwrite a target that changed independently."
        />
      </div>
    </>
  );
}

function AppearanceSettings() {
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">APPEARANCE</span>
          <h2>Approved light product theme</h2>
          <p>
            The current beta uses the approved light Sims Mod Health design system for consistent evidence scanning and accessibility.
          </p>
        </div>
      </div>
      <Panel className="settings-card appearance-preview">
        <div className="appearance-swatch appearance-swatch--primary" />
        <div className="appearance-swatch appearance-swatch--mint" />
        <div className="appearance-swatch appearance-swatch--amber" />
        <div className="appearance-swatch appearance-swatch--indigo" />
        <div>
          <strong>Light / Porcelain</strong>
          <span>Current product theme</span>
        </div>
      </Panel>
    </>
  );
}

function PathRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="settings-path-row">
      <span>{label}</span>
      <code>{value}</code>
    </div>
  );
}

function PolicyCard({ title, detail }: { title: string; detail: string }) {
  return (
    <Panel className="settings-card">
      <div className="settings-policy-icon" aria-hidden="true">✓</div>
      <h3>{title}</h3>
      <p>{detail}</p>
    </Panel>
  );
}

function versionLabel(candidate?: InstallationCandidate) {
  if (!candidate) return "Patch unknown";
  if (candidate.version.status === "available") {
    return "Patch " + candidate.version.version.normalized;
  }
  return "Patch unknown";
}

function sourceLabel(source: string) {
  if (source === "knownDocuments") return "Documents";
  if (source === "oneDriveFallback") return "OneDrive Documents";
  if (source === "manual") return "Manual path";
  return source;
}
