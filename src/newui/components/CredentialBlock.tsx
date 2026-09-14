import { useEffect, useState } from "react";
import { api, isCommandError, type CredentialDto, type StrengthDto } from "../../lib/api";
import { SecretField } from "../../components/SecretField";
import { TotpDisplay } from "../../components/TotpDisplay";
import { Tooltip } from "../../components/Tooltip";
import { Icon } from "../../components/Icon";
import { PaneSurface } from "./PaneSurface";
import { DetailField } from "./DetailField";
import { CopyButton } from "./CopyButton";
import { strengthSegments, strengthTone } from "../lib/credentialLayout";

const TONE_CLASS: Record<"danger" | "warning" | "strong", string> = {
  danger: "bg-[var(--danger)]",
  warning: "bg-[var(--warning)]",
  strong: "bg-green-600 dark:bg-green-400",
};

// One credential: username, reprompt-gated secret, TOTP, custom fields (hidden branch also
// reprompt-gated), remarks and a live strength meter - built from the plaintext already fetched
// for the selected site (D-17, HEALTH-02 partial). Shows no breach or reuse row: both are
// full-vault sweeps with no per-item backend command (checklist §5). When `expanded` is false,
// only the header row renders and everything else stays unmounted, so a collapsed credential's
// secret is never sitting in the DOM (T-08-15).
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

  // Deliberately no confirmation dialog here - the delete is a soft delete, fully restorable
  // from Trash, matching SiteDetail.tsx's existing behaviour exactly (immediate action on
  // click). Site deletion keeps its own window.confirm inside SiteEditor - not duplicated here.
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
    <PaneSurface className="space-y-2 p-3">
      <div className="flex items-center gap-2">
        <span className="min-w-0 flex-1 truncate">{credential.username ?? "(no username)"}</span>
        {credential.username && <CopyButton text={credential.username} tooltip="Copy username" />}
        {credential.reprompt && (
          <span className="flex items-center gap-1 rounded border bg-[var(--surface)] border-[var(--border)] px-2 py-0.5 text-xs">
            <Icon name="lock-fill" /> Reprompt
          </span>
        )}
        <Tooltip text="Edit this credential">
          <button
            type="button"
            onClick={onEdit}
            className="rounded px-1 py-0.5 hover:bg-[var(--surface)]"
          >
            <Icon name="pencil-fill" />
          </button>
        </Tooltip>
        <Tooltip text="Move this credential to Trash">
          <button
            type="button"
            onClick={handleDelete}
            className="rounded px-1 py-0.5 text-[var(--danger)] hover:bg-[var(--surface)]"
          >
            <Icon name="trash" />
          </button>
        </Tooltip>
        <Tooltip text={expanded ? "Collapse this credential" : "Expand this credential"}>
          <button
            type="button"
            onClick={onToggle}
            className="rounded px-1 py-0.5 hover:bg-[var(--surface)]"
          >
            <Icon name={expanded ? "chevron-up" : "chevron-down"} />
          </button>
        </Tooltip>
      </div>

      {error && <p className="text-sm text-red-400">{error}</p>}

      {expanded && (
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <SecretField secret={credential.secret} reprompt={credential.reprompt} />
          </div>

          {credential.hasTotp && <TotpDisplay credentialId={credential.id} />}

          {credential.customFields.map((f, i) => (
            <DetailField
              key={i}
              label={f.label}
              value={f.value}
              fieldType={f.fieldType}
              reprompt={credential.reprompt}
            />
          ))}

          {credential.remarks !== null && (
            <div>
              <p className="text-xs font-semibold uppercase tracking-widest opacity-70">Remarks</p>
              <p className="whitespace-pre-wrap break-words text-sm">{credential.remarks}</p>
            </div>
          )}

          {strength && (
            <div className="flex items-center gap-2 text-sm">
              <div className="flex gap-1">
                {Array.from({ length: 5 }, (_, i) => {
                  const lit = i < strengthSegments(strength.score);
                  const tone = strengthTone(strength.score);
                  return (
                    <span
                      key={i}
                      className={`h-1.5 w-6 rounded-full ${lit ? TONE_CLASS[tone] : "bg-[var(--border)]"}`}
                    />
                  );
                })}
              </div>
              <span className="text-xs opacity-70">{strength.label}</span>
              <Tooltip text="Open the full password health dashboard">
                <button
                  type="button"
                  onClick={onOpenHealth}
                  className="text-xs text-[var(--accent)] hover:underline"
                >
                  View full health
                </button>
              </Tooltip>
            </div>
          )}
        </div>
      )}
    </PaneSurface>
  );
}
