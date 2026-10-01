// Single choke point for the icon font (bootstrap-icons, imported once in App.css) - mirrors how
// Bitwarden always renders `<i class="bwi bwi-x">`, never a raw glyph name inline at each call
// site. `name` is a bootstrap-icons glyph name without the `bi-` prefix (e.g. "house", "trash").
// aria-hidden: every call site already has an accessible name via Tooltip/title/visible text.
export function Icon({ name, className = "" }: { name: string; className?: string }) {
  // N2: `inline-block` is load-bearing, not decorative - a plain `<i>` is `display: inline`, and
  // CSS transforms (Tailwind's `animate-spin` rotates via `transform`) are a no-op on inline
  // non-replaced elements. Without this, Spinner's icon renders motionless despite the animation
  // class being applied and the keyframes running - it was the login spinner's "doesn't actually
  // spin" bug, silently affecting any future `animate-*`/rotate usage on an icon too.
  return <i className={`bi bi-${name} inline-block ${className}`} aria-hidden="true" />;
}
