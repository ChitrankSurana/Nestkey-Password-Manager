// Single choke point for the icon font (bootstrap-icons, imported once in App.css) - mirrors how
// Bitwarden always renders `<i class="bwi bwi-x">`, never a raw glyph name inline at each call
// site. `name` is a bootstrap-icons glyph name without the `bi-` prefix (e.g. "house", "trash").
// aria-hidden: every call site already has an accessible name via Tooltip/title/visible text.
export function Icon({ name, className = "" }: { name: string; className?: string }) {
  return <i className={`bi bi-${name} ${className}`} aria-hidden="true" />;
}
