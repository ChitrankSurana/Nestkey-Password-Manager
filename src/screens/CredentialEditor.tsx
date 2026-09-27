import { useState, useEffect, FormEvent } from "react";
import { api, isCommandError, CredentialDto, CustomField, CustomFieldType } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";
import { PasswordGeneratorPanel } from "../components/PasswordGeneratorPanel";
import { MasterPasswordPrompt } from "../components/MasterPasswordPrompt";
import { isRepromptRequired, markRepromptSuccess } from "../lib/reprompt";

let nextFieldKey = 0;
type EditableField = CustomField & { key: number };

export function CredentialEditor({
  siteId,
  existing,
  onClose,
  onSaved,
}: {
  siteId: number;
  existing: CredentialDto | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [username, setUsername] = useState(existing?.username ?? "");
  const [secret, setSecret] = useState(existing?.secret ?? "");
  const [remarks, setRemarks] = useState(existing?.remarks ?? "");
  const [repromptExempt, setRepromptExempt] = useState(existing?.repromptExempt ?? false);
  const [fields, setFields] = useState<EditableField[]>(
    () => (existing?.customFields ?? []).map((f) => ({ ...f, key: nextFieldKey++ })),
  );
  const [totpSecretInput, setTotpSecretInput] = useState("");
  const [removeTotp, setRemoveTotp] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // B3/N22: a brand-new credential's password is fresh input the user is actively typing/
  // generating, nothing to protect yet - starts revealed. An existing credential's already-stored
  // secret starts masked, matching how it's shown everywhere else in the app; revealing it here
  // goes through the same reprompt gate as SecretField.
  const [passwordRevealed, setPasswordRevealed] = useState(existing === null);
  const [showReauth, setShowReauth] = useState(false);
  // B3: the generator panel is one of the biggest contributors to the modal's overflow height:
  // collapsed by default when editing (the password is presumably already fine), open by default
  // when adding (about to pick one).
  const [generatorOpen, setGeneratorOpen] = useState(existing === null);

  function addField() {
    setFields((fs) => [...fs, { key: nextFieldKey++, label: "", value: "", fieldType: "text" }]);
  }
  function removeField(key: number) {
    setFields((fs) => fs.filter((f) => f.key !== key));
  }
  function updateField(key: number, patch: Partial<CustomField>) {
    setFields((fs) => fs.map((f) => (f.key === key ? { ...f, ...patch } : f)));
  }

  function togglePasswordReveal() {
    if (passwordRevealed) {
      setPasswordRevealed(false);
      return;
    }
    if (isRepromptRequired(repromptExempt)) {
      setShowReauth(true);
      return;
    }
    setPasswordRevealed(true);
  }

  const [algorithm, setAlgorithm] = useState("simple");

  useEffect(() => {
    api.getConfig().then((c) => setAlgorithm(c.strengthAlgorithm)).catch(() => {});
  }, []);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      const customFields: CustomField[] = fields
        .filter((f) => f.label.trim() !== "")
        .map(({ label, value, fieldType }) => ({ label, value, fieldType }));
      let credentialId: number;
      if (existing) {
        await api.updateCredential(existing.id, username || null, secret, remarks || null, customFields, repromptExempt);
        credentialId = existing.id;
      } else {
        credentialId = await api.addCredential(siteId, username || null, secret, remarks || null, customFields, repromptExempt);
      }
      if (totpSecretInput.trim() !== "") {
        await api.setTotpSecret(credentialId, totpSecretInput.trim());
      } else if (removeTotp) {
        await api.clearTotpSecret(credentialId);
      }
      onSaved();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="fixed inset-0 z-[60] bg-black/50 flex items-center justify-center p-4">
      {showReauth && (
        <MasterPasswordPrompt
          title="Re-enter your master password"
          onResult={(ok) => {
            setShowReauth(false);
            if (ok) {
              markRepromptSuccess();
              setPasswordRevealed(true);
            }
          }}
        />
      )}
      {/* B3: no scroll container here previously - content taller than the viewport (the always-
          mounted generator panel, custom fields, TOTP) pushed the Save/Cancel footer off-screen with
          no way to reach it. max-h + overflow-y-auto keeps the footer reachable regardless of
          window size. */}
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-80 max-h-[calc(100vh-2rem)] overflow-y-auto space-y-3 rounded bg-[var(--surface)] p-4 text-[var(--fg)]"
      >
        <h2 className="text-lg font-semibold">{existing ? "Edit credential" : "Add credential"}</h2>
        <label className="block">
          <span className="text-sm">Username (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoFocus
            title="Username or email for this login"
          />
        </label>
        <label className="block">
          <span className="text-sm">Password</span>
          <div className="mt-1 flex items-center gap-1">
            <input
              className="flex-1 rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
              type={passwordRevealed ? "text" : "password"}
              value={secret}
              onChange={(e) => setSecret(e.target.value)}
              title="The password to store, encrypted"
            />
            <button
              type="button"
              onClick={togglePasswordReveal}
              className="rounded border px-2 py-1 text-xs border-[var(--border)]"
              title={passwordRevealed ? "Hide password" : "Show password"}
            >
              {passwordRevealed ? "Hide" : "Show"}
            </button>
          </div>
        </label>

        <div>
          <button
            type="button"
            onClick={() => setGeneratorOpen((v) => !v)}
            className="text-xs opacity-70 hover:opacity-100"
          >
            {generatorOpen ? "▾ Hide generator" : "▸ Generate a password"}
          </button>
          {generatorOpen && (
            <PasswordGeneratorPanel value={secret} onChange={setSecret} algorithm={algorithm} disabled={submitting} />
          )}
        </div>

        <label className="block">
          <span className="text-sm">Remarks (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
            value={remarks}
            onChange={(e) => setRemarks(e.target.value)}
            title="Any notes about this login"
          />
        </label>

        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-sm">Custom fields (optional)</span>
            <Tooltip text="Add a custom field">
              <button type="button" onClick={addField} className="rounded border px-2 py-0.5 text-sm border-[var(--border)]">
                <Icon name="plus-lg" /> Add
              </button>
            </Tooltip>
          </div>
          {fields.map((f) => (
            <div key={f.key} className="flex items-center gap-1">
              <input
                className="w-24 rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)] text-sm"
                placeholder="Label"
                value={f.label}
                onChange={(e) => updateField(f.key, { label: e.target.value })}
              />
              {f.fieldType === "boolean" ? (
                <input
                  type="checkbox"
                  className="flex-1"
                  checked={f.value === "true"}
                  onChange={(e) => updateField(f.key, { value: e.target.checked ? "true" : "false" })}
                />
              ) : (
                <input
                  className="flex-1 rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)] text-sm"
                  type={f.fieldType === "hidden" ? "password" : "text"}
                  placeholder="Value"
                  value={f.value}
                  onChange={(e) => updateField(f.key, { value: e.target.value })}
                />
              )}
              <select
                className="rounded border px-1 py-1 bg-[var(--bg)] border-[var(--border)] text-sm"
                value={f.fieldType}
                onChange={(e) => updateField(f.key, { fieldType: e.target.value as CustomFieldType })}
              >
                <option value="text">Text</option>
                <option value="hidden">Hidden</option>
                <option value="boolean">Boolean</option>
              </select>
              <button type="button" title="Remove field" onClick={() => removeField(f.key)} className="px-1">
                <Icon name="x-lg" />
              </button>
            </div>
          ))}
        </div>

        <label className="block">
          <span className="text-sm">
            {existing?.hasTotp ? "Replace authenticator (TOTP) secret" : "Authenticator (TOTP) secret (optional)"}
          </span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)] font-mono"
            value={totpSecretInput}
            onChange={(e) => {
              setTotpSecretInput(e.target.value);
              if (e.target.value !== "") setRemoveTotp(false);
            }}
            placeholder={existing?.hasTotp ? "Configured - leave blank to keep" : "e.g. JBSWY3DPEHPK3PXP"}
            title="Base32 secret from the site's 'set up 2FA' screen"
          />
          {existing?.hasTotp && (
            <label className="mt-1 flex items-center gap-2">
              <input
                type="checkbox"
                checked={removeTotp}
                onChange={(e) => {
                  setRemoveTotp(e.target.checked);
                  if (e.target.checked) setTotpSecretInput("");
                }}
              />
              <span className="text-sm">Remove authenticator secret</span>
            </label>
          )}
        </label>

        {/* B7: every credential is gated by the master password by default now - this checkbox is
            the opt-OUT, not the opt-in the old "Require master password to view" copy described. */}
        <label className="flex items-center gap-2">
          <input type="checkbox" checked={repromptExempt} onChange={(e) => setRepromptExempt(e.target.checked)} />
          <span className="text-sm" title="Skip the master-password prompt when revealing or copying this item - not recommended">
            Don't require master password to view
          </span>
        </label>

        {error && <p className="text-sm text-red-400">{error}</p>}
        <div className="flex justify-end space-x-2">
          <Tooltip text="Cancel">
            <button type="button" onClick={onClose} className="px-3 py-1">
              <Icon name="x" /> Cancel
            </button>
          </Tooltip>
          <Tooltip text="Save this credential">
            <button
              type="submit"
              disabled={submitting || secret.trim() === ""}
              className="rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50"
            >
              <Icon name="check-lg" /> Save
            </button>
          </Tooltip>
        </div>
      </form>
    </div>
  );
}
