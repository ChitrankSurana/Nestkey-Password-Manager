import { useEffect, useState } from "react";
import { api, isCommandError, GenerateRequest, StrengthDto } from "../lib/api";
import { Tooltip } from "./Tooltip";
import { Icon } from "./Icon";
import { IconCopy } from "../makeui/icons";
import { MK_COPY_BUTTON_CLASS } from "../makeui/components/CopyButton";
import { simpleEntropyBits, entropyBand } from "../lib/strength";

type GenMethod = "random" | "memorable" | "pattern" | "pronounceable";

const BAR_COLORS = ["bg-red-600", "bg-orange-500", "bg-yellow-500", "bg-lime-500", "bg-green-600"];

export function PasswordGeneratorPanel({
  value,
  onChange,
  algorithm,
  disabled,
  variant,
}: {
  value: string;
  onChange: (secret: string) => void;
  algorithm: string;
  disabled?: boolean;
  variant?: "make";
}) {
  const [genMethod, setGenMethod] = useState<GenMethod>("random");
  const [length, setLength] = useState(20);
  const [classes, setClasses] = useState({ uppercase: true, lowercase: true, digits: true, symbols: true });
  const [words, setWords] = useState(4);
  const [separator, setSeparator] = useState("-");
  const [template, setTemplate] = useState("Cvcv-####");
  const [genError, setGenError] = useState<string | null>(null);
  const [zxcvbn, setZxcvbn] = useState<StrengthDto | null>(null);

  useEffect(() => {
    if (algorithm !== "zxcvbn" || value === "") {
      setZxcvbn(null);
      return;
    }
    const id = setTimeout(() => {
      api.analyzeStrength(value).then(setZxcvbn).catch(() => {});
    }, 250);
    return () => clearTimeout(id); // debounce: cancel the pending call on each keystroke
  }, [value, algorithm]);

  const meter =
    algorithm === "zxcvbn"
      ? zxcvbn ?? { score: 0, label: "…" }
      : (() => {
          const bits = simpleEntropyBits(value);
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
      onChange(await api.generatePassword(req));
    } catch (err) {
      setGenError(isCommandError(err) ? err.message : "Could not generate a password.");
    }
  }

  return (
    <div className="space-y-2 rounded border border-[var(--border)] p-2">
      <label className="block">
        <span className="text-sm">Generate method</span>
        <select
          className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
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
              className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
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
              className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
              value={words}
              onChange={(e) => setWords(Number(e.target.value))}
            />
          </label>
          <label className="block">
            <span className="text-sm">Separator</span>
            <input
              className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
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
              className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
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
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
            value={length}
            onChange={(e) => setLength(Number(e.target.value))}
          />
        </label>
      )}

      <div>
        <div className="h-2 w-full overflow-hidden rounded bg-[var(--surface)]">
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
            disabled={disabled}
            className="rounded bg-[var(--accent)] px-3 py-1 disabled:opacity-50"
          >
            <Icon name="dice-5" /> Generate
          </button>
        </Tooltip>
        <Tooltip text="Copy to clipboard (auto-clears after 30s)">
          <button
            type="button"
            onClick={() => api.copyToClipboard(value)}
            disabled={disabled || value === ""}
            title="Copy"
            className={variant === "make" ? MK_COPY_BUTTON_CLASS : "px-3 py-1 disabled:opacity-50"}
          >
            {variant === "make" ? (
              <IconCopy size={12} />
            ) : (
              <>
                <Icon name="clipboard" /> Copy
              </>
            )}
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
