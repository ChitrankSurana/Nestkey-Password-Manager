import { useState } from "react";
import { api, isCommandError, type CredentialDto, type FolderDto, type SiteDto } from "../../lib/api";
import { siteDeleteMessage } from "../../lib/confirm";
import { confirmDelete } from "../../lib/deleteGuard";
import { splitCredentials, otherLoginsLabel } from "../../newui/lib/credentialLayout";
import { CredentialBlock } from "./CredentialBlock";
import { DetailField } from "./DetailField";
import { CopyButton } from "./CopyButton";
import { IconChevron, IconFolder, IconLock, IconShield, IconStar, IconTrash } from "../icons";
import { itemTypeConfig, reservedField, RESERVED_LABEL } from "../lib/vaultSelectors";

// Detail column, ported from figma-make/src/App.tsx:660-926 onto real data. Login renders the
// primary/others credential repeater (same D-08 split newui's DetailPanel uses); note/card/
// identity read their sealed fields off the site's sole credential row via the reserved-label
// convention (D-H) instead of dedicated columns. The History tab and last-modified/created-at
// rows are dropped here - SiteDto carries no timestamps and there is no history table yet
// (that's step 7, B1-4).
export function DetailPanel({
  site,
  folders,
  credentials,
  credentialsLoading,
  error,
  onEditSite,
  onAddCredential,
  onEditCredential,
  onCredentialDeleted,
  onOpenHealth,
  onSiteDeleted,
  onFavoriteToggled,
  onBack,
}: {
  site: SiteDto | null;
  folders: FolderDto[];
  credentials: CredentialDto[] | null;
  credentialsLoading: boolean;
  error: string | null;
  onEditSite: () => void;
  onAddCredential: () => void;
  onEditCredential: (credential: CredentialDto) => void;
  onCredentialDeleted: () => void;
  onOpenHealth: () => void;
  onSiteDeleted: () => void;
  onFavoriteToggled: () => void;
  /** Below md: (B3-8), the detail column is the only thing on screen - this is its way back to
   * the list. Meaningless at md: and up, where both panes are already visible side by side. */
  onBack: () => void;
}) {
  const [othersOpen, setOthersOpen] = useState(false);
  const [expandedOthers, setExpandedOthers] = useState<Set<number>>(new Set());
  const [actionError, setActionError] = useState<string | null>(null);

  if (!site) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 bg-mk-base">
        <div className="w-16 h-16 rounded-2xl flex items-center justify-center bg-mk-accent-muted">
          <IconShield size={28} className="text-mk-accent opacity-60" />
        </div>
        <div className="text-center">
          <p className="text-sm font-medium text-mk-faint">Select an item to view details</p>
          <p className="text-xs mt-0.5 text-mk-faint opacity-60">Your vault is encrypted locally</p>
        </div>
      </div>
    );
  }

  const cfg = itemTypeConfig(site.itemType);
  const { Icon } = cfg;
  const folderName = site.folderId !== null ? folders.find((f) => f.id === site.folderId)?.name ?? null : null;

  async function toggleFavorite() {
    if (!site) return;
    try {
      setActionError(null);
      await api.setSiteFavorite(site.id, !site.favorite);
      onFavoriteToggled();
    } catch (err) {
      setActionError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  async function handleDeleteSite() {
    if (!site) return;
    const message = siteDeleteMessage(site.name, credentials?.length ?? null);
    if (!(await confirmDelete("trashItem", message))) return;
    try {
      setActionError(null);
      await api.softDeleteSite(site.id);
      onSiteDeleted();
    } catch (err) {
      setActionError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  function toggleOther(id: number) {
    setExpandedOthers((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  }

  function renderLoginBody() {
    if (credentialsLoading && credentials === null) {
      return <p className="text-sm text-mk-faint">Loading credentials…</p>;
    }
    if (credentials === null) return null;
    if (credentials.length === 0) {
      return (
        <div className="p-3 rounded-xl border border-mk-border bg-mk-card text-center">
          <p className="text-sm font-medium text-mk-fg2">No credentials yet</p>
          <p className="text-xs mt-1 text-mk-faint">Add a credential to store a username and password.</p>
          <button onClick={onAddCredential} className="mt-3 rounded-lg bg-mk-accent px-3 py-1.5 text-xs font-medium text-white">
            Add credential
          </button>
        </div>
      );
    }
    const { primary, others } = splitCredentials(credentials);
    if (!primary) return null;
    return (
      <div className="space-y-2">
        <CredentialBlock
          credential={primary}
          expanded
          onToggle={() => {}}
          onEdit={() => onEditCredential(primary)}
          onDeleted={onCredentialDeleted}
          onOpenHealth={onOpenHealth}
        />
        {others.length > 0 && (
          <>
            <button onClick={() => setOthersOpen((v) => !v)} className="flex items-center gap-1 rounded px-2 py-1 text-xs text-mk-muted hover:bg-mk-hover">
              {otherLoginsLabel(credentials.length)}
              <IconChevron size={10} down={othersOpen} />
            </button>
            {othersOpen &&
              others.map((c) => (
                <CredentialBlock
                  key={c.id}
                  credential={c}
                  expanded={expandedOthers.has(c.id)}
                  onToggle={() => toggleOther(c.id)}
                  onEdit={() => onEditCredential(c)}
                  onDeleted={onCredentialDeleted}
                  onOpenHealth={onOpenHealth}
                />
              ))}
          </>
        )}
      </div>
    );
  }

  function renderTypeBody() {
    if (site!.itemType === "login") return renderLoginBody();

    if (site!.itemType === "note") {
      const note = reservedField(credentials, RESERVED_LABEL.note);
      return (
        <div className="rounded-xl border p-3 border-mk-border bg-mk-card">
          {note !== null ? (
            <DetailField label="Note" value={note} fieldType="hidden" repromptExempt={credentials?.[0]?.repromptExempt ?? false} />
          ) : (
            <p className="text-sm text-mk-faint">{credentialsLoading ? "Loading…" : "No note content yet."}</p>
          )}
        </div>
      );
    }

    if (site!.itemType === "card") {
      const number = reservedField(credentials, RESERVED_LABEL.cardNumber);
      const expiry = reservedField(credentials, RESERVED_LABEL.cardExpiry);
      const cvv = reservedField(credentials, RESERVED_LABEL.cardCvv);
      const repromptExempt = credentials?.[0]?.repromptExempt ?? false;
      return (
        <div className="rounded-xl border p-3 space-y-3 border-mk-border bg-mk-card">
          {site!.cardBrand && site!.cardLast4 && (
            <p className="text-xs text-mk-muted">
              {site!.cardBrand} ending in {site!.cardLast4}
            </p>
          )}
          {number !== null && <DetailField label="Card Number" value={number} fieldType="hidden" repromptExempt={repromptExempt} />}
          {expiry !== null && (
            <div className="border-t border-mk-border pt-3">
              <DetailField label="Expiration Date" value={expiry} fieldType="text" repromptExempt={repromptExempt} />
            </div>
          )}
          {cvv !== null && (
            <div className="border-t border-mk-border pt-3">
              <DetailField label="Security Code (CVV)" value={cvv} fieldType="hidden" repromptExempt={repromptExempt} />
            </div>
          )}
          {number === null && expiry === null && cvv === null && (
            <p className="text-sm text-mk-faint">{credentialsLoading ? "Loading…" : "No card details saved yet."}</p>
          )}
        </div>
      );
    }

    // identity
    const fullName = reservedField(credentials, RESERVED_LABEL.fullName);
    return (
      <div className="rounded-xl border p-3 border-mk-border bg-mk-card">
        {fullName !== null ? (
          <DetailField label="Full Name" value={fullName} fieldType="text" repromptExempt={false} />
        ) : (
          <p className="text-sm text-mk-faint">{credentialsLoading ? "Loading…" : "No identity details saved yet."}</p>
        )}
      </div>
    );
  }

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-mk-base">
      <div className="h-14 px-5 flex items-center justify-between flex-shrink-0 border-b border-mk-border">
        <div className="flex items-center gap-3 min-w-0">
          <button onClick={onBack} className="md:hidden -ml-1 mr-1 p-1 rounded-lg text-mk-muted hover:bg-mk-hover flex-shrink-0" title="Back to your vault">
            <IconChevron size={16} style={{ transform: "rotate(180deg)" }} />
          </button>
          <div className={`w-9 h-9 rounded-xl flex items-center justify-center flex-shrink-0 ${cfg.bg}`}>
            <Icon size={16} className={cfg.fg} />
          </div>
          <div className="min-w-0">
            <h1 className="text-sm font-semibold truncate text-mk-fg">{site.name}</h1>
            <p className="text-xs text-mk-faint">{cfg.label}</p>
          </div>
        </div>
        <div className="flex items-center gap-1.5 flex-shrink-0">
          <button onClick={toggleFavorite} title={site.favorite ? "Unfavorite" : "Favorite"} className={`p-1.5 rounded-lg ${site.favorite ? "text-mk-yellow" : "text-mk-faint"}`}>
            <IconStar size={14} filled={site.favorite} />
          </button>
          {site.itemType === "login" && (
            <button onClick={onAddCredential} className="px-3 py-1.5 rounded-lg text-xs font-medium bg-mk-accent text-white">
              Add credential
            </button>
          )}
          <button onClick={onEditSite} className="px-3 py-1.5 rounded-lg text-xs font-medium text-mk-fg2 bg-mk-badge hover:opacity-80">
            Edit
          </button>
          <button onClick={handleDeleteSite} title="Delete" className="w-7 h-7 flex items-center justify-center rounded-lg text-mk-red hover:bg-mk-badge">
            <IconTrash size={14} />
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-5 py-4 space-y-5">
        {error && <p className="text-sm text-mk-red">{error}</p>}
        {actionError && <p className="text-sm text-mk-red">{actionError}</p>}

        {/* B7: every item type is gated by the master password by default now, and the banner
            follows that - shown whenever at least one credential is NOT exempt, for every item
            type, not just logins (secure notes and cards hold sealed secrets too). */}
        {credentials?.some((c) => !c.repromptExempt) && (
          <div className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg w-max text-[10px] uppercase font-bold tracking-wide bg-mk-accent-muted text-mk-accent-light border border-mk-accent-border">
            <IconLock size={10} /> Protected — requires master password
          </div>
        )}

        {renderTypeBody()}

        {site.itemType === "login" && site.url && (
          <div className="group p-3 rounded-xl border border-mk-border bg-mk-card">
            <p className="text-[10px] font-medium uppercase tracking-widest mb-1 text-mk-faint">Website</p>
            <div className="flex items-center gap-2">
              <a href={site.url} target="_blank" rel="noopener noreferrer" className="flex-1 text-sm break-all hover:underline text-mk-accent">
                {site.url}
              </a>
              <CopyButton text={site.url} title="Copy URL" />
            </div>
          </div>
        )}

        {site.notes && (
          <div className="p-3 rounded-xl border border-mk-border bg-mk-card">
            <p className="text-[10px] font-medium uppercase tracking-widest mb-1 text-mk-faint">Notes</p>
            <p className="text-sm whitespace-pre-wrap break-words text-mk-fg2">{site.notes}</p>
          </div>
        )}

        {folderName && (
          <div className="pt-2 border-t border-mk-border">
            <p className="text-[10px] font-medium uppercase tracking-widest mb-1 text-mk-faint">Folder</p>
            <div className="flex items-center gap-1.5">
              <IconFolder size={12} className="text-mk-faint" />
              <span className="text-sm text-mk-muted">{folderName}</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
