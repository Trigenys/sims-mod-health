import { useState } from "react";
import { AppShell } from "./components/layout/AppShell";
import { LibraryPage } from "./features/library/LibraryPage";
import { ModDetailPage } from "./features/library/ModDetailPage";
import {
  findLibraryItem,
  type LibraryItem
} from "./features/library/library.fixture";
import type { LibraryRegistryState } from "./features/library/LibraryStateNotice";
import { OverviewPage } from "./features/overview/OverviewPage";

type Surface = "Overview" | "Library" | "Detail";

function initialSurface(): Surface {
  const value = new URLSearchParams(window.location.search).get("surface");
  if (value === "library") return "Library";
  if (value === "detail") return "Detail";
  return "Overview";
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
  const [selectedItem, setSelectedItem] = useState<LibraryItem>(
    findLibraryItem(params.get("mod") ?? "mccc")
  );
  const registryState = initialRegistryState();

  const navigate = (label: string) => {
    if (label === "Overview" || label === "Library") {
      setSurface(label);
    }
  };

  const openItem = (item: LibraryItem) => {
    setSelectedItem(item);
    setSurface("Detail");
  };

  const activeItem = surface === "Overview" ? "Overview" : "Library";

  return (
    <AppShell activeItem={activeItem} onNavigate={navigate}>
      {surface === "Overview" && <OverviewPage />}
      {surface === "Library" && (
        <LibraryPage
          onOpenItem={openItem}
          registryState={registryState}
        />
      )}
      {surface === "Detail" && (
        <ModDetailPage
          item={selectedItem}
          onBack={() => setSurface("Library")}
        />
      )}
    </AppShell>
  );
}

export default App;
