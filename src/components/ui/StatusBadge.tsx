export type StatusTone = "healthy" | "update" | "warning" | "muted" | "danger";

const symbols: Record<StatusTone, string> = {
  healthy: "✓",
  update: "↑",
  warning: "!",
  muted: "?",
  danger: "×"
};

type StatusBadgeProps = {
  tone: StatusTone;
  children: string;
};

export function StatusBadge({ tone, children }: StatusBadgeProps) {
  return (
    <span className={`status-badge status-badge--${tone}`}>
      <span className="status-badge__symbol" aria-hidden="true">{symbols[tone]}</span>
      <span>{children}</span>
    </span>
  );
}
