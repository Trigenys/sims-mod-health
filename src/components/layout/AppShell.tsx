import type { ReactNode } from "react";
import { Sidebar } from "./Sidebar";

type AppShellProps = {
  activeItem?: string;
  onNavigate?: (label: string) => void;
  children: ReactNode;
};

export function AppShell({
  activeItem = "Overview",
  onNavigate,
  children
}: AppShellProps) {
  return (
    <div className="app-shell">
      <Sidebar activeItem={activeItem} onNavigate={onNavigate} />
      <main className="workspace">{children}</main>
    </div>
  );
}
