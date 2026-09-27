import { useState, type FormEvent } from "react";
import { api, isCommandError, type CredentialDto, type CustomField, type CustomFieldType, type FolderDto, type SiteDto } from "../../lib/api";
import { confirmDialog, siteDeleteMessage } from "../../lib/confirm";
import { PasswordGeneratorPanel } from "../../components/PasswordGeneratorPanel";
import { RESERVED_LABEL } from "../lib/vaultSelectors";

type ItemType = "login" | "note" | "card" | "identity";
const TYPE_TABS: { value: ItemType; label: string }[] = [
  { value: "login", label: "Login" },
  { value: "note", label: "Secure Note" },
  { value: "card", label: "Payment Card" },
  { value: "identity", label: "Identity" },
];

let nextFieldKey = 0;
type EditableField = CustomField & { key: number };

// B3-6: the Add/Edit drawer, ported from figma-make/src/App.tsx:1953-2188 onto real data. One
// item = one site + (for non-login types) exactly one credential row holding the sealed
// reserved-label fields (D-H, gap (c) from the step 4/5 log). Generate routes to
// api.generatePassword, never Math.random (B3-6's own note). Item type is fixed once created
// (backend gap (d): changing an existing login's type doesn't re-validate its credentials'
// labels) - the type tabs only appear in "add" mode.
export function ItemDrawer({
  mode,
  existingSite,
  existingCredentials,
  folders,
  onClose,
  onSaved,
  onDeleted,
}: {
  mode: "add" | "edit";
  existingSite: SiteDto | null;
  existingCredentials: CredentialDto[] | null;
  folders: FolderDto[];
  onClose: () => void;
  onSaved: () => void;
  onDeleted?: () => void;
}) {
  const primaryCredential = existingCredentials?.[0] ?? null;
  const itemType: ItemType = (existingSite?.itemType as ItemType) ?? "login";

  const [type, setType] = useState<ItemType>(itemType);
  const [name, setName] = useState(existingSite?.name ?? "");
  const [url, setUrl] = useState(existingSite?.url ?? "");
  const [notes, setNotes] = useState(existingSite?.notes ?? "");
  const [folderId, setFolderId] = useState<number | null>(existingSite?.folderId ?? null);
  const [favorite, setFavorite] = useState(existingSite?.favorite ?? false);

  // Login fields - only collected on create (editing an existing login's credential happens via
  // CredentialBlock's own Edit button, so this drawer never touches credentials for an edit of an
  // existing login site).
  const [username, setUsername] = useState("");
  const [secret, setSecret] = useState("");
  const [remarks, setRemarks] = useState("");
  const [reprompt, setReprompt] = useState(false);
  const [fields, setFields] = useState<EditableField[]>([]);
  const [totpSecretInput, setTotpSecretInput] = useState("");

  // Non-login sealed fields, prefilled from the sole credential's custom fields when editing.
  const [noteBody, setNoteBody] = useState(primaryCredential?.customFields.find((f) => f.label === RESERVED_LABEL.note)?.value ?? "");
  const [cardBrand, setCardBrand] = useState(existingSite?.cardBrand ?? "");
  const [cardLast4, setCardLast4] = useState(existingSite?.cardLast4 ?? "");
  const [cardNumber, setCardNumber] = useState(primaryCredential?.customFields.find((f) => f.label === RESERVED_LABEL.cardNumber)?.value ?? "");
  const [cardExpiry, setCardExpiry] = useState(primaryCredential?.customFields.find((f) => f.label === RESERVED_LABEL.cardExpiry)?.value ?? "");
  const [cardCvv, setCardCvv] = useState(primaryCredential?.customFields.find((f) => f.label === RESERVED_LABEL.cardCvv)?.value ?? "");
  const [fullName, setFullName] = useState(primaryCredential?.customFields.find((f) => f.label === RESERVED_LABEL.fullName)?.value ?? "");

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

  function sealedFieldsFor(t: ItemType): CustomField[] {
    if (t === "note") {
      return noteBody.trim() === "" ? [] : [{ label: RESERVED_LABEL.note, value: noteBody, fieldType: "hidden" }];
    }
    if (t === "card") {
      const out: CustomField[] = [];
      if (cardNumber.trim() !== "") out.push({ label: RESERVED_LABEL.cardNumber, value: cardNumber, fieldType: "hidden" });
      if (cardExpiry.trim() !== "") out.push({ label: RESERVED_LABEL.cardExpiry, value: cardExpiry, fieldType: "text" });
      if (cardCvv.trim() !== "") out.push({ label: RESERVED_LABEL.cardCvv, value: cardCvv, fieldType: "hidden" });
      return out;
    }
    if (t === "identity") {
      return fullName.trim() === "" ? [] : [{ label: RESERVED_LABEL.fullName, value: fullName, fieldType: "text" }];
    }
    return [];
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      if (mode === "add") {
        const siteId = await api.addSite(name, url.trim() || null, notes.trim() || null);
        if (favorite) await api.setSiteFavorite(siteId, true);
        if (type !== "login") await api.setSiteItemType(siteId, type);
        if (folderId !== null) await api.setSiteFolder(siteId, folderId);
        if (type === "card" && (cardBrand.trim() !== "" || cardLast4.trim() !== "")) {
          await api.setSiteCardHint(siteId, cardBrand.trim() || null, cardLast4.trim() || null);
        }
        if (type === "login") {
          const customFields: CustomField[] = fields
            .filter((f) => f.label.trim() !== "")
            .map(({ label, value, fieldType }) => ({ label, value, fieldType }));
          const credentialId = await api.addCredential(siteId, username.trim() || null, secret, remarks.trim() || null, customFields, reprompt);
          if (totpSecretInput.trim() !== "") await api.setTotpSecret(credentialId, totpSecretInput.trim());
        } else {
          await api.addCredential(siteId, null, "", null, sealedFieldsFor(type), false);
        }
      } else if (existingSite) {
        await api.updateSite(existingSite.id, name, url.trim() || null, notes.trim() || null);
        if (favorite !== existingSite.favorite) await api.setSiteFavorite(existingSite.id, favorite);
        if (folderId !== existingSite.folderId) await api.setSiteFolder(existingSite.id, folderId);
        if (type === "card" && (cardBrand !== (existingSite.cardBrand ?? "") || cardLast4 !== (existingSite.cardLast4 ?? ""))) {
          await api.setSiteCardHint(existingSite.id, cardBrand.trim() || null, cardLast4.trim() || null);
        }
        if (type !== "login") {
          const sealed = sealedFieldsFor(type);
          if (primaryCredential) {
            await api.updateCredential(primaryCredential.id, null, "", null, sealed, false);
          } else if (sealed.length > 0) {
            await api.addCredential(existingSite.id, null, "", null, sealed, false);
          }
        }
      }
      onSaved();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleDelete() {
    if (!existingSite) return;
    if (!(await confirmDialog(siteDeleteMessage(existingSite.name, existingCredentials?.length ?? null)))) return;
    setSubmitting(true);
    setError(null);
    try {
      await api.softDeleteSite(existingSite.id);
      onDeleted?.();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="fixed inset-0 z-[60] bg-black/50 flex items-center justify-end">
      <form onSubmit={handleSubmit} className="h-full w-full max-w-md overflow-y-auto space-y-4 bg-mk-card border-l border-mk-border p-5 text-mk-fg">
        <h2 className="text-lg font-semibold">{mode === "edit" ? `Edit — ${existingSite?.name ?? ""}` : "New Item"}</h2>

        {mode === "add" && (
          <div className="flex gap-1 rounded-lg bg-mk-badge p-1">
            {TYPE_TABS.map((t) => (
              <button
                key={t.value}
                type="button"
                onClick={() => setType(t.value)}
                className={`flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition-colors ${
                  type === t.value ? "bg-mk-accent text-white" : "text-mk-muted hover:text-mk-fg2"
                }`}
              >
                {t.label}
              </button>
            ))}
          </div>
        )}
        {mode === "edit" && <p className="text-xs text-mk-faint">Type: {TYPE_TABS.find((t) => t.value === type)?.label} (fixed after creation)</p>}

        <label className="block">
          <span className="text-sm text-mk-fg2">Name</span>
          <input
            className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg"
            value={name}
            onChange={(e) => setName(e.target.value)}
            autoFocus
            title="Item name"
          />
        </label>

        <div className="flex gap-3">
          <label className="flex-1 block">
            <span className="text-sm text-mk-fg2">Folder</span>
            <select
              className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg"
              value={folderId ?? ""}
              onChange={(e) => setFolderId(e.target.value === "" ? null : Number(e.target.value))}
            >
              <option value="">No folder</option>
              {folders.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.name}
                </option>
              ))}
            </select>
          </label>
          <label className="flex items-center gap-2 mt-6">
            <input type="checkbox" checked={favorite} onChange={(e) => setFavorite(e.target.checked)} />
            <span className="text-sm text-mk-fg2">Favorite</span>
          </label>
        </div>

        {type === "login" && (
          <>
            <label className="block">
              <span className="text-sm text-mk-fg2">Website URL (optional)</span>
              <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg" value={url} onChange={(e) => setUrl(e.target.value)} />
            </label>
            {mode === "add" && (
              <>
                <label className="block">
                  <span className="text-sm text-mk-fg2">Username (optional)</span>
                  <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg" value={username} onChange={(e) => setUsername(e.target.value)} />
                </label>
                <label className="block">
                  <span className="text-sm text-mk-fg2">Password</span>
                  <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg font-mono" value={secret} onChange={(e) => setSecret(e.target.value)} />
                </label>
                <PasswordGeneratorPanel value={secret} onChange={setSecret} algorithm="simple" disabled={submitting} />
                <label className="block">
                  <span className="text-sm text-mk-fg2">Remarks (optional)</span>
                  <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg" value={remarks} onChange={(e) => setRemarks(e.target.value)} />
                </label>

                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm text-mk-fg2">Custom fields (optional)</span>
                    <button type="button" onClick={addField} className="rounded-lg border px-2 py-0.5 text-xs border-mk-border text-mk-muted">
                      + Add
                    </button>
                  </div>
                  {fields.map((f) => (
                    <div key={f.key} className="flex items-center gap-1">
                      <input
                        className="w-24 rounded-lg border px-2 py-1 text-xs bg-mk-input border-mk-border text-mk-fg"
                        placeholder="Label"
                        value={f.label}
                        onChange={(e) => updateField(f.key, { label: e.target.value })}
                      />
                      {f.fieldType === "boolean" ? (
                        // J1: was a free-text box for every field type, so typing "Yes" stored the
                        // literal string "Yes" instead of "true"/"false" - readers compare strictly
                        // against "true" (DetailField.tsx), so anything else silently reads as No.
                        // Matches the checkbox CredentialEditor.tsx already uses for this type.
                        <input
                          type="checkbox"
                          className="flex-1"
                          checked={f.value === "true"}
                          onChange={(e) => updateField(f.key, { value: e.target.checked ? "true" : "false" })}
                        />
                      ) : (
                        <input
                          className="flex-1 rounded-lg border px-2 py-1 text-xs bg-mk-input border-mk-border text-mk-fg"
                          type={f.fieldType === "hidden" ? "password" : "text"}
                          placeholder="Value"
                          value={f.value}
                          onChange={(e) => updateField(f.key, { value: e.target.value })}
                        />
                      )}
                      <select
                        className="rounded-lg border px-1 py-1 text-xs bg-mk-input border-mk-border text-mk-fg"
                        value={f.fieldType}
                        onChange={(e) => updateField(f.key, { fieldType: e.target.value as CustomFieldType })}
                      >
                        <option value="text">Text</option>
                        <option value="hidden">Hidden</option>
                        <option value="boolean">Boolean</option>
                      </select>
                      <button type="button" onClick={() => removeField(f.key)} className="px-1 text-mk-faint">
                        ✕
                      </button>
                    </div>
                  ))}
                </div>

                <label className="block">
                  <span className="text-sm text-mk-fg2">Authenticator (TOTP) secret (optional)</span>
                  <input
                    className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg font-mono"
                    value={totpSecretInput}
                    onChange={(e) => setTotpSecretInput(e.target.value)}
                    placeholder="e.g. JBSWY3DPEHPK3PXP"
                  />
                </label>

                <label className="flex items-center gap-2">
                  <input type="checkbox" checked={reprompt} onChange={(e) => setReprompt(e.target.checked)} />
                  <span className="text-sm text-mk-fg2">Require master password to view</span>
                </label>
              </>
            )}
            {mode === "edit" && (
              <p className="text-xs text-mk-faint">Username, password, and other credential fields are edited from the credential's own Edit button.</p>
            )}
          </>
        )}

        {type === "note" && (
          <label className="block">
            <span className="text-sm text-mk-fg2">Note</span>
            <textarea
              className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg"
              rows={6}
              value={noteBody}
              onChange={(e) => setNoteBody(e.target.value)}
            />
          </label>
        )}

        {type === "card" && (
          <>
            <div className="flex gap-3">
              <label className="flex-1 block">
                <span className="text-sm text-mk-fg2">Card brand (optional)</span>
                <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg" value={cardBrand} onChange={(e) => setCardBrand(e.target.value)} placeholder="Visa" />
              </label>
              <label className="w-28 block">
                <span className="text-sm text-mk-fg2">Last 4</span>
                <input
                  className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg font-mono"
                  value={cardLast4}
                  maxLength={4}
                  onChange={(e) => setCardLast4(e.target.value.replace(/\D/g, "").slice(0, 4))}
                />
              </label>
            </div>
            <label className="block">
              <span className="text-sm text-mk-fg2">Card number</span>
              <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg font-mono" value={cardNumber} onChange={(e) => setCardNumber(e.target.value)} />
            </label>
            <label className="block">
              <span className="text-sm text-mk-fg2">Expiration date</span>
              <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg" value={cardExpiry} onChange={(e) => setCardExpiry(e.target.value)} placeholder="MM/YY" />
            </label>
            <label className="block">
              <span className="text-sm text-mk-fg2">Security code (CVV)</span>
              <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg font-mono" value={cardCvv} onChange={(e) => setCardCvv(e.target.value)} />
            </label>
          </>
        )}

        {type === "identity" && (
          <label className="block">
            <span className="text-sm text-mk-fg2">Full name</span>
            <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg" value={fullName} onChange={(e) => setFullName(e.target.value)} />
          </label>
        )}

        {type !== "login" && (
          // N7: was gated on mode === "add" too, so edit mode never showed this input even though
          // `notes` is still read from the existing site and submitted on save (:123) - the value
          // round-tripped but was un-editable from this drawer.
          <label className="block">
            <span className="text-sm text-mk-fg2">Notes (optional)</span>
            <input className="mt-1 w-full rounded-lg border px-2 py-1.5 bg-mk-input border-mk-border text-mk-fg" value={notes} onChange={(e) => setNotes(e.target.value)} />
          </label>
        )}

        {error && <p className="text-sm text-mk-red">{error}</p>}

        <div className="flex items-center justify-between pt-2">
          {mode === "edit" ? (
            <button type="button" onClick={handleDelete} disabled={submitting} className="rounded-lg bg-mk-red hover:bg-mk-red-hover px-3 py-1.5 text-xs font-medium text-white disabled:opacity-50">
              Delete
            </button>
          ) : (
            <span />
          )}
          <div className="flex gap-2">
            <button type="button" onClick={onClose} className="px-3 py-1.5 text-xs font-medium text-mk-muted">
              Cancel
            </button>
            <button
              type="submit"
              disabled={submitting || name.trim() === ""}
              className="rounded-lg bg-mk-accent px-4 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
            >
              {mode === "edit" ? "Save Changes" : "Add Item"}
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
