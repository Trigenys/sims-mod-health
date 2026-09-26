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
          label="Search library"
          placeholder={`Search ${indexedCount} mods & CC`}
        />
        <Button
          variant="primary"
          onClick={onScan}
          disabled={scanDisabled || scanning}
        >
          {scanning ? "Scanning…" : "Scan now"}
        </Button>
      </div>
    </header>
  );
}
