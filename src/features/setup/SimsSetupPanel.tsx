import { useEffect, useMemo, useState } from "react";
import { Button } from "../../components/ui/Button";
import { Panel } from "../../components/ui/Panel";
import { useI18n } from "../../i18n/i18n";
import type { GameContentInstallation } from "../game-content/gameContent.types";
import { setupGateway, type SetupGateway } from "./setup.gateway";
import type { UserInstallationCandidate } from "./setup.types";

type SimsSetupPanelProps = {
  gateway?: SetupGateway;
  onScanComplete: () => Promise<void> | void;
};

export function SimsSetupPanel({
  gateway = setupGateway,
  onScanComplete
}: SimsSetupPanelProps) {
  const { t } = useI18n();
  const [game, setGame] = useState<GameContentInstallation | null>(null);
  const [userRoot, setUserRoot] = useState<UserInstallationCandidate | null>(null);
  const [detecting, setDetecting] = useState(true);
  const [working, setWorking] = useState<"game" | "user" | "mods" | "scan" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const detect = async () => {
    setDetecting(true);
    setError(null);
    try {
      const snapshot = await gateway.detect();
      setGame(snapshot.gameInventory.installations[0] ?? null);
      setUserRoot(
        snapshot.userInstallations.find((candidate) => candidate.modsAvailable)
          ?? snapshot.userInstallations[0]
          ?? null
      );
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setDetecting(false);
    }
  };

  useEffect(() => {
    void detect();
  }, [gateway]);

  const gameVersion = game?.build.version?.normalized ?? null;
  const modsReady = userRoot?.modsAvailable === true;
  const ready = Boolean(game && gameVersion && userRoot && modsReady);

  const provider = useMemo(() => {
    if (!game) return null;
    if (game.provider === "ea_app") return "EA app";
    if (game.provider === "steam") return "Steam";
    return t("Selected game folder");
  }, [game, t]);

  const chooseGame = async () => {
    setWorking("game");
    setError(null);
    try {
      const selected = await gateway.chooseGameFolder(t("Choose your The Sims 4 game folder"));
      if (selected) setGame(selected);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setWorking(null);
    }
  };

  const chooseUser = async () => {
    setWorking("user");
    setError(null);
    try {
      const selected = await gateway.chooseUserFolder(t("Choose your Sims 4 user folder"));
      if (selected) setUserRoot(selected);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setWorking(null);
    }
  };

  const chooseMods = async () => {
    setWorking("mods");
    setError(null);
    try {
      const selected = await gateway.chooseModsFolder(t("Choose your Mods folder"));
      if (selected) setUserRoot(selected);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setWorking(null);
    }
  };

  const scan = async () => {
    if (!ready || !userRoot) return;
    setWorking("scan");
    setError(null);
    try {
      await gateway.scan(userRoot.root);
      await onScanComplete();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setWorking(null);
    }
  };

  return (
    <Panel className="setup-panel" aria-labelledby="setup-title">
      <div className="setup-panel__intro">
        <div className="setup-panel__icon" aria-hidden="true">✓</div>
        <div>
          <span className="section-kicker">{t("FIRST SETUP")}</span>
          <h1 id="setup-title">{t("Connect your Sims 4 installation")}</h1>
          <p>
            {t("Before scanning your mods, Sims Mod Health needs to know where your game and Sims 4 files are. We try to find them automatically first.")}
          </p>
        </div>
        <Button variant="text" onClick={() => void detect()} disabled={detecting || working !== null}>
          {detecting ? t("Detecting…") : t("Detect again")}
        </Button>
      </div>

      <div className="setup-checklist" aria-label={t("Setup checklist")}>
        <SetupStep
          ready={Boolean(game)}
          title={t("Game installation")}
          detail={
            game
              ? t("Found with {{provider}}", { provider: provider ?? t("your selected folder") })
              : t("We could not find the installed game automatically.")
          }
          secondary={game?.installRoot ?? null}
          actionLabel={game ? t("Choose a different game folder") : t("Choose game folder")}
          busy={working === "game"}
          onAction={chooseGame}
        />

        <SetupStep
          ready={Boolean(gameVersion)}
          title={t("Game version")}
          detail={
            gameVersion
              ? t("Version {{version}} detected", { version: gameVersion })
              : game
                ? t("We found the game, but could not read its version. Choose the correct game folder or detect again.")
                : t("The game version will appear after the game is found.")
          }
          actionLabel={game && !gameVersion ? t("Choose game folder") : null}
          busy={working === "game"}
          onAction={chooseGame}
        />

        <SetupStep
          ready={Boolean(game)}
          title={t("Installed packs")}
          detail={
            game
              ? game.packs.length === 0
                ? t("No extra packs were found. Base game only is okay.")
                : t("{{count}} installed packs found", { count: game.packs.length })
              : t("Packs will be checked after the game is found.")
          }
        />

        <SetupStep
          ready={Boolean(userRoot)}
          title={t("Sims 4 user folder")}
          detail={
            userRoot
              ? t("Your saves, settings and Mods folder were found.")
              : t("We could not find your Sims 4 user folder automatically.")
          }
          secondary={userRoot?.root ?? null}
          actionLabel={userRoot ? t("Choose a different user folder") : t("Choose Sims 4 folder")}
          busy={working === "user"}
          onAction={chooseUser}
        />

        <SetupStep
          ready={modsReady}
          title={t("Mods folder")}
          detail={
            modsReady
              ? t("Mods folder ready to scan.")
              : userRoot
                ? t("The Sims 4 folder was found, but the Mods folder is missing.")
                : t("The Mods folder will be checked after your Sims 4 user folder is found.")
          }
          secondary={modsReady ? userRoot?.modsRoot ?? null : null}
          actionLabel={!modsReady ? t("Choose Mods folder") : null}
          busy={working === "mods"}
          onAction={chooseMods}
        />
      </div>

      {error && (
        <div className="setup-panel__error" role="alert">
          <strong>{t("We could not complete that step.")}</strong>
          <span>{error}</span>
        </div>
      )}

      <div className="setup-panel__footer">
        <div>
          <strong>
            {ready
              ? t("Everything needed for a useful scan is ready.")
              : t("Complete the missing steps before scanning.")}
          </strong>
          <span>
            {ready
              ? t("The scan will use your game version, installed packs and Mods folder together.")
              : t("This prevents unknown game information from being shown as if it were a real result.")}
          </span>
        </div>
        <Button onClick={() => void scan()} disabled={!ready || working !== null || detecting}>
          {working === "scan" ? t("Scanning…") : t("Scan my mods")}
        </Button>
      </div>
    </Panel>
  );
}

function SetupStep({
  ready,
  title,
  detail,
  secondary,
  actionLabel,
  busy,
  onAction
}: {
  ready: boolean;
  title: string;
  detail: string;
  secondary?: string | null;
  actionLabel?: string | null;
  busy?: boolean;
  onAction?: () => Promise<void>;
}) {
  const { t } = useI18n();

  return (
    <article className={ready ? "setup-step setup-step--ready" : "setup-step"}>
      <div className="setup-step__state" aria-hidden="true">{ready ? "✓" : "!"}</div>
      <div className="setup-step__copy">
        <div>
          <strong>{title}</strong>
          <span className={ready ? "setup-step__badge setup-step__badge--ready" : "setup-step__badge"}>
            {ready ? t("Found") : t("To complete")}
          </span>
        </div>
        <p>{detail}</p>
        {secondary && <small title={secondary}>{secondary}</small>}
      </div>
      {actionLabel && onAction && (
        <Button variant="text" onClick={() => void onAction()} disabled={busy}>
          {busy ? t("Opening…") : actionLabel}
        </Button>
      )}
    </article>
  );
}
