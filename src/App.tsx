import { useState } from "react";
import { AppShell } from "./components/layout/AppShell";
import { DiscoverPage } from "./features/discover/DiscoverPage";
import { HealthPage, type HealthTab } from "./features/health/HealthPage";
import { LibraryPage } from "./features/library/LibraryPage";
import { ModDetailPage } from "./features/library/ModDetailPage";
import { findLibraryItem } from "./features/library/library.gateway";
import type { LibraryItem } from "./features/library/library.fixture";
import type { LibraryRegistryState } from "./features/library/LibraryStateNotice";
import { OverviewPage } from "./features/overview/OverviewPage";
import { SettingsPage } from "./features/settings/SettingsPage";

type Surface =
  | "Overview"
  | "Library"
  | "Detail"
  | "Health"
  | "Discover"
  | "Settings";

function initialSurface(): Surface {
  const value = new URLSearchParams(window.location.search).get("surface");
  if (value === "library") return "Library";
  if (value === "detail") return "Detail";
  if (value === "health" || value === "diagnostics") return "Health";
  if (value === "discover") return "Discover";
  if (value === "settings") return "Settings";
  return "Overview";
}

function initialHealthTab(): HealthTab {
  const params = new URLSearchParams(window.location.search);
  if (params.get("surface") === "diagnostics") return "diagnostics";

  const value = params.get("tab");
  if (
    value === "updates" ||
    value === "conflicts" ||
    value === "diagnostics" ||
    value === "recovery"
  ) {
    return value;
  }
  return "all";
}

function initialRegistryState(): LibraryRegistryState {
  const value = new URLSearchParams(window.location.search).get("state");
  if (value === "offline" || value === "partial" || value === "failure") {
    return value;
  }
  return "ready";
}

function App() {
  const params = new URLSearchParams(window.location.search);
  const [surface, setSurface] = useState<Surface>(initialSurface);
  const [selectedItem, setSelectedItem] = useState<LibraryItem | null>(
    params.get("visual") === "detail"
      ? findLibraryItem(params.get("mod") ?? "mccc")
      : null
  );
  const registryState = initialRegistryState();

  const navigate = (label: string) => {
    if (
      label === "Overview" ||
      label === "Library" ||
      label === "Health" ||
      label === "Discover" ||
      label === "Settings"
    ) {
      setSurface(label);
    }
  };

  const openItem = (item: LibraryItem) => {
    setSelectedItem(item);
    setSurface("Detail");
  };

  const activeItem = surface === "Detail" ? "Library" : surface;

  return (
    <AppShell activeItem={activeItem} onNavigate={navigate}>
      {surface === "Overview" && <OverviewPage />}
      {surface === "Health" && (
        <HealthPage
          initialTab={initialHealthTab()}
          onOpenLibrary={() => setSurface("Library")}
        />
      )}
      {surface === "Discover" && <DiscoverPage />}
      {surface === "Settings" && <SettingsPage />}
      {surface === "Library" && (
        <LibraryPage
          onOpenItem={openItem}
          registryState={registryState}
        />
      )}
      {surface === "Detail" && selectedItem && (
        <ModDetailPage
          item={selectedItem}
          onBack={() => setSurface("Library")}
        />
      )}
      {surface === "Detail" && !selectedItem && (
        <LibraryPage
          onOpenItem={openItem}
          registryState={registryState}
        />
      )}
    </AppShell>
  );
}

export default App;
