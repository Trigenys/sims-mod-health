import type { HTMLAttributes, ReactNode } from "react";

type PanelProps = HTMLAttributes<HTMLElement> & {
  as?: "section" | "article" | "aside";
  children: ReactNode;
};

export function Panel({ as: Element = "section", className = "", children, ...props }: PanelProps) {
  return (
    <Element className={["panel", className].filter(Boolean).join(" ")} {...props}>
      {children}
    </Element>
  );
}
