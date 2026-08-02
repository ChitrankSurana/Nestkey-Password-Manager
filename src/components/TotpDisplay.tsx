import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { toast } from "../lib/toast";
import { Icon } from "./Icon";

// Polled once a second while a TOTP-configured row is visible - the backend computes the code
// from the seed (kept out of the WebView entirely), this just displays what comes back.
export function TotpDisplay({ credentialId }: { credentialId: number }) {
  const [totp, setTotp] = useState<{ code: string; secondsRemaining: number } | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function tick() {
      try {
        const result = await api.totpCode(credentialId);
        if (!cancelled) setTotp(result);
      } catch {
        if (!cancelled) setTotp(null);
      }
    }
    tick();
    const id = setInterval(tick, 1000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [credentialId]);

  if (!totp) return null;

  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="font-mono tracking-widest">{totp.code}</span>
      <span className="opacity-60 text-xs w-6">{totp.secondsRemaining}s</span>
      <button
        type="button"
        title="Copy code"
        onClick={() => {
          api.copyToClipboard(totp.code);
          toast("Code copied. It will clear from the clipboard in 30s.");
        }}
        className="rounded px-2 py-0.5 border border-[var(--border)]"
      >
        <Icon name="clipboard" />
      </button>
    </div>
  );
}
