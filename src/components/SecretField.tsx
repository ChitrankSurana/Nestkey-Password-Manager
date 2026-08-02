import { useState } from "react";
import { api } from "../lib/api";
import { toast } from "../lib/toast";
import { classifyChar } from "../lib/passwordColor";
import { Icon } from "./Icon";

const CLASS_COLOR: Record<"digit" | "symbol", string> = {
  digit: "text-[var(--pw-digit)]",
  symbol: "text-[var(--pw-symbol)]",
};

// Extracted from ViewerVault's inline reveal/copy logic (the only place this app previously
// exposed password reveal+copy) so SiteDetail can use the same behavior on the main vault view.
// Colorizes digits/symbols on reveal (Bitwarden's bit-color-password idea) - letters inherit the
// default text color, only digit/symbol runs get a wrapping span.
export function SecretField({ secret }: { secret: string }) {
  const [revealed, setRevealed] = useState(false);

  return (
    <>
      <span className="flex-1 font-mono">
        {revealed
          ? [...secret].map((ch, i) => {
              const cls = classifyChar(ch);
              return cls ? (
                <span key={i} className={CLASS_COLOR[cls]}>
                  {ch}
                </span>
              ) : (
                ch
              );
            })
          : "••••••••"}
      </span>
      <button
        type="button"
        title={revealed ? "Hide password" : "Show password"}
        onClick={() => setRevealed((r) => !r)}
        className="rounded px-2 py-0.5 border border-[var(--border)]"
      >
        <Icon name={revealed ? "eye-slash-fill" : "eye-fill"} />
      </button>
      <button
        type="button"
        title="Copy password"
        onClick={() => {
          api.copyToClipboard(secret);
          toast("Password copied. It will clear from the clipboard in 30s.");
        }}
        className="rounded px-2 py-0.5 border border-[var(--border)]"
      >
        <Icon name="clipboard" /> Copy
      </button>
    </>
  );
}
