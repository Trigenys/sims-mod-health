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
  const { t, tx } = useI18n();
  const programInstallation = gameInventory.installations[0];

  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("LOCAL INSTALLATION")}</span>
          <h2>{t("Sims 4 & Mods paths")}</h2>
          <p>
            {t("Paths are detected locally. The scanner reads the selected Mods directory without uploading raw files.")}
          </p>
        </div>
        <Button variant="secondary" onClick={() => void onDetect()} disabled={detecting}>
          {detecting ? t("Detecting…") : t("Rescan locations")}
        </Button>
      </div>

      <Panel className="settings-card settings-card--featured">
        <div className="settings-card__header">
          <div>
            <span className="section-kicker">{t("ACTIVE SIMS 4 ROOT")}</span>
            <h3>{active ? tx(sourceLabel(active.source)) : t("No installation detected")}</h3>
          </div>
          <span className={active?.modsAvailable ? "settings-ok" : "settings-muted"}>
            {active?.modsAvailable ? t("Mods folder available") : t("Not available")}
          </span>
        </div>

        <PathRow
          label={t("Sims 4 user folder")}
          value={active?.root ?? t("Run detection in the desktop application")}
        />
        <PathRow
          label={t("Mods folder")}
          value={active?.modsRoot ?? t("No Mods directory detected")}
        />
        <PathRow
          label={t("Game patch")}
          value={
            programInstallation?.build.version?.normalized
              ? t("Patch {{version}}", { version: programInstallation.build.version.normalized })
              : versionLabel(active, t)
          }
        />
        <PathRow
          label={t("Game program folder")}
          value={programInstallation?.installRoot ?? t("No program installation detected")}
        />
        <PathRow
          label={t("Update provider")}
          value={tx(formatProvider(programInstallation?.provider ?? providerCapability?.provider ?? "unknown"))}
        />
        <PathRow
          label={t("Installed packs")}
          value={String(programInstallation?.packs.length ?? 0)}
        />
      </Panel>

      {installations.length > 1 && (
        <Panel className="settings-card">
          <span className="section-kicker">{t("OTHER DETECTED ROOTS")}</span>
          <div className="settings-installations">
            {installations.slice(1).map((item) => (
              <div key={item.root}>
                <strong>{tx(sourceLabel(item.source))}</strong>
                <code>{item.root}</code>
              </div>
            ))}
          </div>
        </Panel>
      )}

      <Panel className="settings-card">
        <span className="section-kicker">{t("UPDATE PROVIDER")}</span>
        <h3>{t("Official provider handoff only")}</h3>
        <p>
          {providerCapability?.detail
            ? tx(providerCapability.detail)
            : t("Provider capability is resolved locally when a game installation is available.")}
          {" "}{t("After an EA app or Steam update, Sims Mod Health rescans local game and pack evidence before declaring success.")}
        </p>
      </Panel>

      <Panel className="settings-card">
        <span className="section-kicker">{t("REGISTRY BEHAVIOR")}</span>
        <h3>{t("Local scan remains the baseline")}</h3>
        <p>
          {t("Registry identity, compatibility, relationships and Discover enrich local evidence. If the Registry is unreachable, the app keeps local inventory, duplicate and diagnostic evidence visible.")}
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
  const { t } = useI18n();
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("PRIVACY")}</span>
          <h2>{t("Checks & privacy")}</h2>
          <p>
            {t("Diagnostic telemetry is explicit opt-in. Raw diagnostic reports and raw mod files are not uploaded automatically.")}
          </p>
        </div>
      </div>

      <Panel className="settings-card settings-toggle-card">
        <div>
          <h3>{t("Redacted diagnostic telemetry")}</h3>
          <p>
            {t("When enabled, only redacted summary metadata is eligible for telemetry. Invalid consent state fails closed.")}
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
          <span>{enabled ? t("On") : t("Off")}</span>
        </label>
      </Panel>

      <div className="settings-grid">
        <PolicyCard
          title={t("Local parsing")}
          detail={t("DBPF, TS4Script and supported diagnostic reports are parsed on-device with bounded readers.")}
        />
        <PolicyCard
          title={t("Registry requests")}
          detail={t("Canonical identity uses constrained artifact metadata and supported fingerprints rather than raw file uploads.")}
        />
        <PolicyCard
          title={t("Evidence language")}
          detail={t("Correlation and potential conflicts remain distinct from deterministic breakage.")}
        />
      </div>
    </>
  );
}

function ScanSettings() {
  const { t } = useI18n();
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("SCANNER")}</span>
          <h2>{t("Scan behavior")}</h2>
          <p>
            {t("The production scanner defaults to incremental work so unchanged files can be skipped safely.")}
          </p>
        </div>
      </div>
      <div className="settings-grid">
        <PolicyCard
          title={t("Incremental by default")}
          detail={t("Known unchanged artifacts reuse persisted evidence; changed files are hashed and re-inspected.")}
        />
        <PolicyCard
          title={t("Bounded inspection")}
          detail={t("Archive, package, script and diagnostic parsing use explicit limits rather than unbounded recursive work.")}
        />
        <PolicyCard
          title={t("No automatic destructive cleanup")}
          detail={t("Health findings are reviewable evidence. Unknown files are not silently removed.")}
        />
      </div>
    </>
  );
}

function RecoverySettings() {
  const { t } = useI18n();
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("RECOVERY & STORAGE")}</span>
          <h2>{t("Safe mutation policy")}</h2>
          <p>
            {t("Supported updates use app-controlled staging and verified restore points.")}
          </p>
        </div>
      </div>
      <div className="settings-grid">
        <PolicyCard
          title={t("Restore points")}
          detail={t("The original artifact is verified before the Mods folder is mutated.")}
        />
        <PolicyCard
          title={t("Persistent journal")}
          detail={t("Interrupted update transactions remain recoverable after restart.")}
        />
        <PolicyCard
          title={t("Rollback guard")}
          detail={t("Rollback refuses to overwrite a target that changed independently.")}
        />
      </div>
    </>
  );
}

function AppearanceSettings() {
  const { t } = useI18n();
  return (
    <>
      <div className="settings-section-heading">
        <div>
          <span className="section-kicker">{t("APPEARANCE")}</span>
          <h2>{t("Approved light product theme")}</h2>
          <p>
            {t("The current beta uses the approved light Sims Mod Health design system for consistent evidence scanning and accessibility.")}
          </p>
        </div>
      </div>
      <Panel className="settings-card appearance-preview">
        <div className="appearance-swatch appearance-swatch--primary" />
        <div className="appearance-swatch appearance-swatch--mint" />
        <div className="appearance-swatch appearance-swatch--amber" />
        <div className="appearance-swatch appearance-swatch--indigo" />
        <div>
          <strong>{t("Light / Porcelain")}</strong>
          <span>{t("Current product theme")}</span>
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

function versionLabel(
  candidate: InstallationCandidate | undefined,
  t: ReturnType<typeof useI18n>["t"]
) {
  if (!candidate) return t("Patch unknown");
  if (candidate.version.status === "available") {
    return t("Patch {{version}}", { version: candidate.version.version.normalized });
  }
  return t("Patch unknown");
}

function sourceLabel(source: string) {
  if (source === "knownDocuments") return "Documents";
  if (source === "oneDriveFallback") return "OneDrive Documents";
  if (source === "manual") return "Manual path";
  return source;
}
