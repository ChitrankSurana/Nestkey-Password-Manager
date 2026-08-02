import { useState, useEffect, FormEvent } from "react";
import { api, isCommandError, CredentialDto, CustomField, CustomFieldType } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";
import { PasswordGeneratorPanel } from "../components/PasswordGeneratorPanel";

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
  const [reprompt, setReprompt] = useState(existing?.reprompt ?? false);
  const [fields, setFields] = useState<EditableField[]>(
    () => (existing?.customFields ?? []).map((f) => ({ ...f, key: nextFieldKey++ })),
  );
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function addField() {
    setFields((fs) => [...fs, { key: nextFieldKey++, label: "", value: "", fieldType: "text" }]);
  }
  function removeField(key: number) {
    setFields((fs) => fs.filter((f) => f.key !== key));
  }
  function updateField(key: number, patch: Partial<CustomField>) {
    setFields((fs) => fs.map((f) => (f.key === key ? { ...f, ...patch } : f)));
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
      if (existing) {
        await api.updateCredential(existing.id, username || null, secret, remarks || null, customFields, reprompt);
      } else {
        await api.addCredential(siteId, username || null, secret, remarks || null, customFields, reprompt);
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
      <form onSubmit={handleSubmit} className="w-full max-w-80 space-y-3 rounded bg-[var(--surface)] p-4 text-[var(--fg)]">
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
          <input
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
            value={secret}
            onChange={(e) => setSecret(e.target.value)}
            title="The password to store, encrypted"
          />
        </label>

        <PasswordGeneratorPanel value={secret} onChange={setSecret} algorithm={algorithm} disabled={submitting} />

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

        <label className="flex items-center gap-2">
          <input type="checkbox" checked={reprompt} onChange={(e) => setReprompt(e.target.checked)} />
          <span className="text-sm" title="Ask for the master password again before revealing or copying this item, even while the vault is unlocked">
            Require master password to view
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
