import type { ReactNode } from "react";

// Shared card/surface wrapper - the codebase never factored this out (every screen inlines its
// own `rounded border ... border-[var(--border)]` classes). Used by the detail column's
// card-shaped blocks (plan 08-04) and anywhere else a card-shaped block needs the three-pane
// chrome to read consistently.
export function PaneSurface({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={`rounded-lg border bg-[var(--surface)] border-[var(--border)] ${className}`}>
      {children}
    </div>
  );
}
