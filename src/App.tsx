import { AppShell } from "./components/layout/AppShell";
import { OverviewPage } from "./features/overview/OverviewPage";

function App() {
  return (
    <AppShell activeItem="Overview">
      <OverviewPage />
    </AppShell>
  );
}

export default App;
