import { useState, useEffect, FormEvent } from "react";
import { api, isCommandError, CredentialDto, GenerateRequest, StrengthDto } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { simpleEntropyBits, entropyBand } from "../lib/strength";

type GenMethod = "random" | "memorable" | "pattern" | "pronounceable";

const BAR_COLORS = ["bg-red-600", "bg-orange-500", "bg-yellow-500", "bg-lime-500", "bg-green-600"];

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
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [genMethod, setGenMethod] = useState<GenMethod>("random");
  const [length, setLength] = useState(20);
  const [classes, setClasses] = useState({ uppercase: true, lowercase: true, digits: true, symbols: true });
  const [words, setWords] = useState(4);
  const [separator, setSeparator] = useState("-");
  const [template, setTemplate] = useState("Cvcv-####");
  const [genError, setGenError] = useState<string | null>(null);

  const [algorithm, setAlgorithm] = useState("simple");
  const [zxcvbn, setZxcvbn] = useState<StrengthDto | null>(null);

  useEffect(() => {
    api.getConfig().then((c) => setAlgorithm(c.strengthAlgorithm)).catch(() => {});
  }, []);

  useEffect(() => {
    if (algorithm !== "zxcvbn" || secret === "") {
      setZxcvbn(null);
      return;
    }
    const id = setTimeout(() => {
      api.analyzeStrength(secret).then(setZxcvbn).catch(() => {});
    }, 250);
    return () => clearTimeout(id); // debounce: cancel the pending call on each keystroke
  }, [secret, algorithm]);

  const meter =
    algorithm === "zxcvbn"
      ? zxcvbn ?? { score: 0, label: "…" }
      : (() => {
          const bits = simpleEntropyBits(secret);
          return { ...entropyBand(bits), bits };
        })();

  async function handleGenerate() {
    setGenError(null);
    try {
      const req: GenerateRequest =
        genMethod === "random"
          ? { method: "random", length, ...classes }
          : genMethod === "memorable"
            ? { method: "memorable", words, separator }
            : genMethod === "pattern"
              ? { method: "pattern", template }
              : { method: "pronounceable", length };
      setSecret(await api.generatePassword(req));
    } catch (err) {
      setGenError(isCommandError(err) ? err.message : "Could not generate a password.");
    }
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      if (existing) {
        await api.updateCredential(existing.id, username || null, secret, remarks || null);
      } else {
        await api.addCredential(siteId, username || null, secret, remarks || null);
      }
      onSaved();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center">
      <form onSubmit={handleSubmit} className="w-80 space-y-3 rounded bg-slate-800 p-4">
        <h2 className="text-lg font-semibold">{existing ? "Edit credential" : "Add credential"}</h2>
        <label className="block">
          <span className="text-sm">Username (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoFocus
            title="Username or email for this login"
          />
        </label>
        <label className="block">
          <span className="text-sm">Password</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
            value={secret}
            onChange={(e) => setSecret(e.target.value)}
            title="The password to store, encrypted"
          />
        </label>

        <div className="space-y-2 rounded border border-slate-600 p-2">
          <label className="block">
            <span className="text-sm">Generate method</span>
            <select
              className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
              value={genMethod}
              onChange={(e) => setGenMethod(e.target.value as GenMethod)}
            >
              <option value="random">Random</option>
              <option value="memorable">Memorable</option>
              <option value="pattern">Pattern</option>
              <option value="pronounceable">Pronounceable</option>
            </select>
          </label>

          {genMethod === "random" && (
            <div className="space-y-1">
              <label className="block">
                <span className="text-sm">Length</span>
                <input
                  type="number"
                  min={1}
                  className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
                  value={length}
                  onChange={(e) => setLength(Number(e.target.value))}
                />
              </label>
              <div className="flex flex-wrap gap-x-3 gap-y-1 text-sm">
                <label className="flex items-center gap-1">
                  <input
                    type="checkbox"
                    checked={classes.uppercase}
                    onChange={(e) => setClasses({ ...classes, uppercase: e.target.checked })}
                  />
                  Uppercase
                </label>
                <label className="flex items-center gap-1">
                  <input
                    type="checkbox"
                    checked={classes.lowercase}
                    onChange={(e) => setClasses({ ...classes, lowercase: e.target.checked })}
                  />
                  Lowercase
                </label>
                <label className="flex items-center gap-1">
                  <input
                    type="checkbox"
                    checked={classes.digits}
                    onChange={(e) => setClasses({ ...classes, digits: e.target.checked })}
                  />
                  Digits
                </label>
                <label className="flex items-center gap-1">
                  <input
                    type="checkbox"
                    checked={classes.symbols}
                    onChange={(e) => setClasses({ ...classes, symbols: e.target.checked })}
                  />
                  Symbols
                </label>
              </div>
            </div>
          )}

          {genMethod === "memorable" && (
            <div className="space-y-1">
              <label className="block">
                <span className="text-sm">Words</span>
                <input
                  type="number"
                  min={1}
                  className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
                  value={words}
                  onChange={(e) => setWords(Number(e.target.value))}
                />
              </label>
              <label className="block">
                <span className="text-sm">Separator</span>
                <input
                  className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
                  value={separator}
                  onChange={(e) => setSeparator(e.target.value)}
                />
              </label>
            </div>
          )}

          {genMethod === "pattern" && (
            <div className="space-y-1">
              <label className="block">
                <span className="text-sm">Template</span>
                <input
                  className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
                  value={template}
                  onChange={(e) => setTemplate(e.target.value)}
                />
              </label>
              <p className="text-xs text-slate-400">C/c = letter (upper/lower), V/v = vowel (upper/lower), # = digit, ? = symbol, \ = literal</p>
            </div>
          )}

          {genMethod === "pronounceable" && (
            <label className="block">
              <span className="text-sm">Length</span>
              <input
                type="number"
                min={1}
                className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
                value={length}
                onChange={(e) => setLength(Number(e.target.value))}
              />
            </label>
          )}

          <div>
            <div className="h-2 w-full overflow-hidden rounded bg-slate-700">
              <div
                className={`h-full ${BAR_COLORS[meter.score]} transition-all`}
                style={{ width: `${(meter.score / 4) * 100}%` }}
              />
            </div>
            <p className="mt-1 text-xs text-slate-400">
              {meter.label}
              {"bits" in meter ? ` (${meter.bits} bits)` : ""}
            </p>
          </div>

          {genError && <p className="text-sm text-red-400">{genError}</p>}

          <div className="flex space-x-2">
            <Tooltip text="Generate a password with the selected method">
              <button
                type="button"
                onClick={handleGenerate}
                disabled={submitting}
                className="rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50"
              >
                Generate
              </button>
            </Tooltip>
            <Tooltip text="Copy to clipboard (auto-clears after 30s)">
              <button
                type="button"
                onClick={() => api.copyToClipboard(secret)}
                disabled={submitting || secret === ""}
                className="px-3 py-1 disabled:opacity-50"
              >
                Copy
              </button>
            </Tooltip>
          </div>
        </div>

        <label className="block">
          <span className="text-sm">Remarks (optional)</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-900 dark:border-slate-600"
            value={remarks}
            onChange={(e) => setRemarks(e.target.value)}
            title="Any notes about this login"
          />
        </label>
        {error && <p className="text-sm text-red-400">{error}</p>}
        <div className="flex justify-end space-x-2">
          <Tooltip text="Cancel">
            <button type="button" onClick={onClose} className="px-3 py-1">
              Cancel
            </button>
          </Tooltip>
          <Tooltip text="Save this credential">
            <button
              type="submit"
              disabled={submitting || secret.trim() === ""}
              className="rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50"
            >
              Save
            </button>
          </Tooltip>
        </div>
      </form>
    </div>
  );
}
