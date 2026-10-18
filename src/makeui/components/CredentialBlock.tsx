import { useEffect, useState } from "react";
import { api, isCommandError, type ChangeLogEntryDto, type CredentialDto, type HistoryEntryDto, type StrengthDto } from "../../lib/api";
import { confirmDelete } from "../../lib/deleteGuard";
import { formatUtcLocal } from "../../lib/time";
import { SecretField } from "../../components/SecretField";
import { TotpDisplay } from "../../components/TotpDisplay";
import { DetailField } from "./DetailField";
import { CopyButton } from "./CopyButton";
import { IconChevron, IconLock, IconTrash } from "../icons";
import { strengthSegments, strengthTone } from "../../newui/lib/credentialLayout";

const TONE_CLASS: Record<"danger" | "warning" | "strong", string> = {
  danger: "bg-mk-red",
  warning: "bg-mk-orange",
  strong: "bg-mk-green",
};

// Wave 4 follow-up (M7.2): raw field names from credential_change_log -> what the Activity tab
// shows. Kept as a lookup rather than titlecasing the raw string so a future field (e.g. a folder
// move) doesn't silently show up under its snake_case column name.
const CHANGE_LOG_FIELD_LABEL: Record<string, string> = {
  username: "Username",
  remarks: "Remarks",
  reprompt_exempt: "Require master password to reveal",
};

function changeLogValueDisplay(value: string | null): string {
  return value === null || value === "" ? "(empty)" : value;
}

// One login credential, restyled from src/newui/components/CredentialBlock.tsx onto the mk-*
// tokens (B3-7: restyle the real component, never reimplement the prototype's mock DetailField).
// Reuses the newui lib's pure strength-segment math rather than re-deriving it.
export function CredentialBlock({
  credential,
  expanded,
  onToggle,
  onEdit,
  onDeleted,
  onOpenHealth,
}: {
  credential: CredentialDto;
  expanded: boolean;
  onToggle: () => void;
  onEdit: () => void;
  onDeleted: () => void;
  onOpenHealth: () => void;
}) {
  const [strength, setStrength] = useState<StrengthDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [history, setHistory] = useState<HistoryEntryDto[] | null>(null);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [changeLog, setChangeLog] = useState<ChangeLogEntryDto[] | null>(null);
  const [changeLogError, setChangeLogError] = useState<string | null>(null);
  // J3 (user request): password history was a disclosure link buried at the bottom of the
  // credential's field list - moved to its own tab alongside Details, per the UAT note asking for
  // this to be a tab under the login item rather than an expandable link. Activity is the Wave 4
  // per-credential change log follow-up (M7.2) - a username-only edit produces no password-history
  // row at all, so it had nowhere to show up before this tab existed.
  const [activeTab, setActiveTab] = useState<"details" | "history" | "activity">("details");
  const historyOpen = activeTab === "history";
  const activityOpen = activeTab === "activity";

  function fetchHistory() {
    if (history !== null) return;
    api
      .listCredentialHistory(credential.id)
      .then(setHistory)
      .catch((err) => setHistoryError(isCommandError(err) ? err.message : "Couldn't load password history."));
  }

  function fetchChangeLog() {
    if (changeLog !== null) return;
    api
      .listCredentialChangeLog(credential.id)
      .then(setChangeLog)
      .catch((err) => setChangeLogError(isCommandError(err) ? err.message : "Couldn't load change history."));
  }

  // J3: history was fetched once (`if (history === null)` above) and never invalidated - saving a
  // new password added a real history row server-side, but this component (rendered with no `key`
  // in DetailPanel, so it keeps its state across a save) kept showing whatever it had cached from
  // before, sometimes just the stale single first-open snapshot. Resetting the cache whenever the
  // live secret changes means the next open refetches instead of showing stale data indefinitely;
  // if the history panel is already open when that happens, refetch immediately rather than
  // leaving it showing the pre-save list until the user happens to collapse and reopen it.
  useEffect(() => {
    setHistory(null);
    setHistoryError(null);
    if (!historyOpen) return;
    api
      .listCredentialHistory(credential.id)
      .then(setHistory)
      .catch((err) => setHistoryError(isCommandError(err) ? err.message : "Couldn't load password history."));
    // Deliberately excludes `historyOpen`/`credential.id` - this must fire only on a secret
    // change, not every time the panel toggles (toggleHistory already handles that case).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [credential.secret]);

  // Same cache-invalidation shape as the history effect above, keyed on the fields the change log
  // actually tracks (username, remarks, reprompt-exempt) instead of the secret - this component has
  // no per-credential `key` (see DetailPanel's `primary` slot), so it survives a save/selection
  // change with stale state unless something here resets it.
  useEffect(() => {
    setChangeLog(null);
    setChangeLogError(null);
    if (!activityOpen) return;
    api
      .listCredentialChangeLog(credential.id)
      .then(setChangeLog)
      .catch((err) => setChangeLogError(isCommandError(err) ? err.message : "Couldn't load change history."));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [credential.id, credential.username, credential.remarks, credential.repromptExempt]);

  async function handleDelete() {
    // M8.1: was the only destructive action in the app with no confirmation at all (site, folder
    // and Empty Trash all route through this same seam) - a stray click on a multi-credential site
    // silently removed one with no way to notice before it happened.
    const label = credential.username || "this credential";
    if (!(await confirmDelete("deleteCredential", `Delete "${label}"? It will move to Trash.`))) return;
    try {
      setError(null);
      await api.softDeleteCredential(credential.id);
      onDeleted();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    }
  }

  useEffect(() => {
    if (!expanded) return;
    let cancelled = false;
    api
      .analyzeStrength(credential.secret)
      .then((result) => {
        if (!cancelled) setStrength(result);
      })
      .catch(() => {
        if (!cancelled) setStrength(null);
      });
    return () => {
      cancelled = true;
    };
  }, [credential.secret, expanded]);

  return (
    <div className="rounded-xl border overflow-hidden border-mk-border bg-mk-card">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-mk-border bg-mk-tint">
        <span className="min-w-0 flex-1 truncate text-sm text-mk-fg2">{credential.username ?? "(no username)"}</span>
        {credential.username && <CopyButton text={credential.username} title="Copy username" />}
        {!credential.repromptExempt && (
          <span className="flex items-center gap-1 text-[10px] font-bold uppercase text-mk-accent-light">
            <IconLock size={9} /> Protected
          </span>
        )}
        <button type="button" onClick={onEdit} className="px-2 py-0.5 rounded text-[10px] font-medium text-mk-muted hover:bg-mk-hover">
          Edit
        </button>
        <button type="button" title="Delete" onClick={handleDelete} className="p-1 rounded text-mk-red hover:bg-mk-hover">
          <IconTrash size={12} />
        </button>
        <button type="button" title={expanded ? "Collapse" : "Expand"} onClick={onToggle} className="p-1 rounded text-mk-muted hover:bg-mk-hover">
          <IconChevron size={12} down={expanded} />
        </button>
      </div>

      {error && <p className="px-3 pt-2 text-xs text-mk-red">{error}</p>}

      {expanded && (
        <div>
          <div className="flex items-center gap-1 px-3 pt-2 border-b border-mk-border">
            {(["details", "history", "activity"] as const).map((tab) => (
              <button
                key={tab}
                type="button"
                onClick={() => {
                  setActiveTab(tab);
                  if (tab === "history") fetchHistory();
                  if (tab === "activity") fetchChangeLog();
                }}
                className={`px-2.5 py-1.5 text-xs font-medium rounded-t-lg border-b-2 -mb-px ${
                  activeTab === tab
                    ? "border-mk-accent text-mk-fg"
                    : "border-transparent text-mk-muted hover:text-mk-fg2"
                }`}
              >
                {tab === "details" ? "Details" : tab === "history" ? "History" : "Activity"}
              </button>
            ))}
          </div>

          {activeTab === "details" ? (
            <div className="p-3 space-y-3">
              <div>
                <p className="text-[10px] font-medium uppercase tracking-widest mb-1 text-mk-faint">Password</p>
                <div className="flex items-center gap-2">
                  <SecretField secret={credential.secret} exempt={credential.repromptExempt} variant="make" />
                </div>
              </div>

              {credential.hasTotp && (
                <div className="px-3 py-2 rounded-lg bg-mk-accent-muted border border-mk-accent-border">
                  <TotpDisplay credentialId={credential.id} variant="make" />
                </div>
              )}

              {credential.customFields.map((f, i) => (
                <div key={i} className="border-t border-mk-border pt-3">
                  <DetailField label={f.label} value={f.value} fieldType={f.fieldType} repromptExempt={credential.repromptExempt} />
                </div>
              ))}

              {credential.remarks !== null && (
                <div className="border-t border-mk-border pt-3">
                  <p className="text-[10px] font-medium uppercase tracking-widest mb-1 text-mk-faint">Remarks</p>
                  <p className="whitespace-pre-wrap break-words text-sm text-mk-fg2">{credential.remarks}</p>
                </div>
              )}

              {strength && (
                <div className="rounded-xl p-3 border border-mk-border bg-mk-badge">
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-mk-muted">Strength</span>
                    <div className="flex items-center gap-1.5">
                      {Array.from({ length: 5 }, (_, i) => {
                        const lit = i < strengthSegments(strength.score);
                        const tone = strengthTone(strength.score);
                        return <span key={i} className={`h-1.5 w-5 rounded-full ${lit ? TONE_CLASS[tone] : "bg-mk-border"}`} />;
                      })}
                      <span className="text-xs font-medium ml-1 text-mk-fg2">{strength.label}</span>
                    </div>
                  </div>
                  <button type="button" onClick={onOpenHealth} className="mt-1 text-xs text-mk-accent hover:underline">
                    View full health
                  </button>
                </div>
              )}
            </div>
          ) : activeTab === "history" ? (
            <div className="p-3 space-y-2">
              {historyError && <p className="text-xs text-mk-red">{historyError}</p>}
              {history === null && !historyError && <p className="text-xs text-mk-faint">Loading…</p>}
              {history !== null && history.length === 0 && <p className="text-xs text-mk-faint">No previous passwords recorded.</p>}
              {history?.map((h, i) => (
                <div key={i} className="flex items-center gap-2">
                  <span className="w-32 shrink-0 text-[10px] text-mk-fg2">{formatUtcLocal(h.changedAt)}</span>
                  <SecretField secret={h.secret} exempt={credential.repromptExempt} variant="make" />
                </div>
              ))}
            </div>
          ) : (
            <div className="p-3 space-y-2">
              {changeLogError && <p className="text-xs text-mk-red">{changeLogError}</p>}
              {changeLog === null && !changeLogError && <p className="text-xs text-mk-faint">Loading…</p>}
              {changeLog !== null && changeLog.length === 0 && <p className="text-xs text-mk-faint">No changes recorded yet.</p>}
              {changeLog?.map((c, i) => (
                <div key={i} className="border-t border-mk-border pt-2 first:border-t-0 first:pt-0">
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-xs font-medium text-mk-fg2">{CHANGE_LOG_FIELD_LABEL[c.field] ?? c.field}</span>
                    <span className="text-[10px] text-mk-fg2">{formatUtcLocal(c.changedAt)}</span>
                  </div>
                  <p className="text-xs text-mk-muted mt-0.5">
                    {changeLogValueDisplay(c.oldValue)} <span className="text-mk-faint">→</span> {changeLogValueDisplay(c.newValue)}
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
