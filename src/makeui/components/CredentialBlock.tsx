import { useEffect, useState } from "react";
import { api, isCommandError, type CredentialDto, type HistoryEntryDto, type StrengthDto } from "../../lib/api";
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
  const [historyOpen, setHistoryOpen] = useState(false);
  const [historyError, setHistoryError] = useState<string | null>(null);

  function toggleHistory() {
    setHistoryOpen((v) => !v);
    if (history === null) {
      api
        .listCredentialHistory(credential.id)
        .then(setHistory)
        .catch((err) => setHistoryError(isCommandError(err) ? err.message : "Couldn't load password history."));
    }
  }

  async function handleDelete() {
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
        {credential.reprompt && (
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
        <div className="p-3 space-y-3">
          <div>
            <p className="text-[10px] font-medium uppercase tracking-widest mb-1 text-mk-faint">Password</p>
            <div className="flex items-center gap-2">
              <SecretField secret={credential.secret} reprompt={credential.reprompt} />
            </div>
          </div>

          {credential.hasTotp && (
            <div className="px-3 py-2 rounded-lg bg-mk-accent-muted border border-mk-accent-border">
              <TotpDisplay credentialId={credential.id} />
            </div>
          )}

          {credential.customFields.map((f, i) => (
            <div key={i} className="border-t border-mk-border pt-3">
              <DetailField label={f.label} value={f.value} fieldType={f.fieldType} reprompt={credential.reprompt} />
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

          <div className="border-t border-mk-border pt-3">
            <button type="button" onClick={toggleHistory} className="flex items-center gap-1 text-xs text-mk-muted hover:text-mk-fg2">
              Password history
              <IconChevron size={10} down={historyOpen} />
            </button>
            {historyOpen && (
              <div className="mt-2 space-y-2">
                {historyError && <p className="text-xs text-mk-red">{historyError}</p>}
                {history === null && !historyError && <p className="text-xs text-mk-faint">Loading…</p>}
                {history !== null && history.length === 0 && <p className="text-xs text-mk-faint">No previous passwords recorded.</p>}
                {history?.map((h, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <span className="w-28 shrink-0 text-[10px] text-mk-faint">{h.changedAt}</span>
                    <SecretField secret={h.secret} reprompt={credential.reprompt} />
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
