import { useState } from "react";
import { api, isCommandError, type CredentialDto, type SiteDto } from "../../lib/api";
import { confirmDialog, siteDeleteMessage } from "../../lib/confirm";
import { PaneSurface } from "./PaneSurface";
import { CredentialBlock } from "./CredentialBlock";
import { CopyButton } from "./CopyButton";
import { Tooltip } from "../../components/Tooltip";
import { Icon } from "../../components/Icon";
import { splitCredentials, otherLoginsLabel } from "../lib/credentialLayout";

// Detail column: site header (name, url, notes) and the credential repeater (D-08). Renders
// purely from props and never fetches - useVault owns the fetch, which is what keeps the
// lazy-decryption invariant checkable in one place (D-09).
export function DetailPanel({
  site,
  credentials,
  credentialsLoading,
  error,
  onEditSite,
  onAddCredential,
  onEditCredential,
  onCredentialDeleted,
  onOpenHealth,
  onBack,
  onSiteDeleted,
}: {
  site: SiteDto | null;
  credentials: CredentialDto[] | null;
  credentialsLoading: boolean;
  error: string | null;
  onEditSite: () => void;
  onAddCredential: () => void;
  onEditCredential: (credential: CredentialDto) => void;
  onCredentialDeleted: () => void;
  onOpenHealth: () => void;
  onBack: () => void;
  onSiteDeleted: () => void;
}) {
  const [othersOpen, setOthersOpen] = useState(false);
  const [expandedOthers, setExpandedOthers] = useState<Set<number>>(new Set());
  const [deleteError, setDeleteError] = useState<string | null>(null);

  if (!site) return null;

  // Same isCommandError/generic-fallback convention CredentialBlock's own handleDelete uses -
  // credentials?.length ?? null makes "still loading" the honest "unknown" branch rather than
  // lying about a count of zero.
  async function handleDeleteSite() {
    if (!site) return;
    const message = siteDeleteMessage(site.name, credentials?.length ?? null);
    if (!(await confirmDialog(message))) return;
    try {
      setDeleteError(null);
      await api.softDeleteSite(site.id);
      onSiteDeleted();
    } catch (err) {
      setDeleteError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  function toggleOther(id: number) {
    setExpandedOthers((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  function renderBody() {
    if (credentialsLoading && credentials === null) {
      return <p className="text-sm opacity-70">Loading credentials…</p>;
    }
    if (credentials === null) {
      return null;
    }
    if (credentials.length === 0) {
      return (
        <PaneSurface className="p-4 text-center">
          <p className="text-sm font-semibold">No credentials yet</p>
          <p className="mt-1 text-sm opacity-70">
            Add a credential to store a username and password for this site.
          </p>
          <Tooltip text="Add a credential to this site">
            <button
              onClick={onAddCredential}
              className="mt-3 rounded bg-[var(--accent)] px-3 py-1 text-sm"
            >
              <Icon name="plus-lg" /> Add credential
            </button>
          </Tooltip>
        </PaneSurface>
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
            <Tooltip text={othersOpen ? "Hide other logins" : "Show other logins"}>
              <button
                onClick={() => setOthersOpen((v) => !v)}
                className="flex items-center gap-1 rounded px-2 py-1 text-sm hover:bg-[var(--surface)]"
              >
                {otherLoginsLabel(credentials.length)}
                <Icon name={othersOpen ? "chevron-up" : "chevron-down"} />
              </button>
            </Tooltip>
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

  return (
    <div className="flex-1 overflow-y-auto p-4">
      {/* Below 640px the detail column is the only thing on screen (D-18), so it needs a way
          back to the list - meaningless at the two wider tiers where both panes are already
          visible, hence sm:hidden. */}
      <Tooltip text="Back to your vault list">
        <button
          type="button"
          onClick={onBack}
          className="sm:hidden mb-4 flex items-center gap-1 rounded px-2 py-1 text-sm hover:bg-[var(--surface)]"
        >
          <Icon name="chevron-left" /> Back
        </button>
      </Tooltip>

      <div className="mb-4">
        <div className="flex items-start justify-between gap-2">
          <h2 className="text-xl font-semibold">{site.name}</h2>
          <div className="flex shrink-0 items-center gap-1">
            <Tooltip text="Edit site">
              <button
                onClick={onEditSite}
                className="rounded px-2 py-1 text-sm hover:bg-[var(--surface)]"
              >
                <Icon name="pencil-fill" /> Edit site
              </button>
            </Tooltip>
            <Tooltip text="Add a credential to this site">
              <button
                onClick={onAddCredential}
                className="rounded bg-[var(--accent)] px-2 py-1 text-sm"
              >
                <Icon name="plus-lg" /> Add credential
              </button>
            </Tooltip>
            <Tooltip text="Delete this site and any credentials saved for it">
              <button
                onClick={handleDeleteSite}
                aria-label="Delete site"
                className="rounded bg-[var(--danger)] hover:bg-[var(--danger-hover)] px-2 py-1 text-sm"
              >
                <Icon name="trash" />
              </button>
            </Tooltip>
          </div>
        </div>
        <div className="mt-1 flex items-center gap-2 text-sm">
          {site.url ? (
            <>
              <a
                href={site.url}
                target="_blank"
                rel="noreferrer"
                className="truncate text-[var(--accent)]"
              >
                {site.url}
              </a>
              <CopyButton text={site.url} tooltip="Copy URL" />
            </>
          ) : (
            <span className="opacity-70">(no URL)</span>
          )}
        </div>
      </div>

      {site.notes !== null && (
        <div className="mb-4 border-t border-[var(--border)] pt-3">
          <p className="text-xs font-semibold uppercase tracking-widest opacity-70">Notes</p>
          <p className="whitespace-pre-wrap break-words text-sm">{site.notes}</p>
        </div>
      )}

      {error && <p className="mb-4 text-sm text-red-400">{error}</p>}
      {deleteError && <p className="mb-4 text-sm text-red-400">{deleteError}</p>}

      {renderBody()}
    </div>
  );
}
