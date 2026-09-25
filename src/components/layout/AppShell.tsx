import type { ReactNode } from "react";
import { Sidebar } from "./Sidebar";

type AppShellProps = {
  activeItem?: string;
  children: ReactNode;
};

export function AppShell({ activeItem = "Overview", children }: AppShellProps) {
  return (
    <div className="app-shell">
      <Sidebar activeItem={activeItem} />
      <main className="workspace">{children}</main>
    </div>
  );
}
