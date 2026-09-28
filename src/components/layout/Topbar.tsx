import { useI18n } from "../../i18n/I18nProvider";
import { Button } from "../ui/Button";
import { SearchField } from "../ui/SearchField";

type TopbarProps = {
  gameVersion: string;
  platform: string;
  indexedCount: number;
  onScan?: () => void | Promise<void>;
  scanning?: boolean;
  scanDisabled?: boolean;
};

export function Topbar({
  gameVersion,
  platform,
  indexedCount,
  onScan,
  scanning = false,
  scanDisabled = false
}: TopbarProps) {
  const { t } = useI18n();

  return (
    <header className="topbar">
      <div>
        <span className="context-label">THE SIMS 4</span>
        <div className="version-row">
          <strong>{gameVersion}</strong>
          <span>{platform}</span>
        </div>
      </div>

      <div className="topbar-actions">
        <SearchField
          label={t("topbar.searchLabel")}
          placeholder={t("topbar.searchPlaceholder", { count: indexedCount })}
        />
        <Button
          variant="primary"
          onClick={onScan}
          disabled={scanDisabled || scanning}
        >
          {scanning ? t("topbar.scanning") : t("topbar.scan")}
        </Button>
      </div>
    </header>
  );
}
