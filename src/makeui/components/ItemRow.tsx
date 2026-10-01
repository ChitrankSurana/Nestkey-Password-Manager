import type { SiteDto } from "../../lib/api";
import { IconStar } from "../icons";
import { itemTypeConfig } from "../lib/vaultSelectors";

// One vault item row: type-colored icon tile, name, subtitle (username for logins, card
// brand/last4 for cards, type label otherwise), favorite star. Ported from
// figma-make/src/App.tsx:563-595 (ItemRow) - same shape, real SiteDto instead of the mock
// VaultItem, and no per-row credential count (that needs the lazy per-site fetch, D-09).
export function ItemRow({ site, selected, onClick }: { site: SiteDto; selected: boolean; onClick: () => void }) {
  const cfg = itemTypeConfig(site.itemType);
  const { Icon } = cfg;

  const subtitle =
    site.itemType === "card"
      ? site.cardBrand && site.cardLast4
        ? `${site.cardBrand} ••${site.cardLast4}`
        : "Payment Card"
      : site.itemType === "note"
        ? "Secure Note"
        : site.itemType === "identity"
          ? "Identity"
          : (site.url ?? "Login");

  return (
    <button
      onClick={onClick}
      className={`w-full flex items-center gap-3 px-3 py-2.5 border-b border-mk-border transition-colors duration-100 ${
        selected ? "bg-mk-accent-muted border-l-2 border-l-mk-accent" : "border-l-2 border-l-transparent hover:bg-mk-hover"
      }`}
    >
      <div className={`w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 ${cfg.bg}`}>
        <Icon size={14} className={cfg.fg} />
      </div>
      <div className="flex-1 min-w-0 text-left">
        <div className={`text-sm font-medium truncate ${selected ? "text-mk-fg" : "text-mk-fg2"}`}>{site.name}</div>
        <div className="text-xs truncate text-mk-faint">{subtitle}</div>
      </div>
      {site.favorite && <IconStar size={11} filled className="text-mk-yellow flex-shrink-0 opacity-70" />}
    </button>
  );
}
