import { api } from "../../lib/api";
import { toast } from "../../lib/toast";
import { Tooltip } from "../../components/Tooltip";
import { Icon } from "../../components/Icon";

// Tooltip-wrapped copy control for non-secret values, routed through the one clipboard seam that
// implements the 30-second "clear only if unchanged" auto-clear on both the desktop (Rust) and
// web (browser) transports - matches SecretField's own copy handler verbatim. Secrets are
// SecretField's job, not this component's - they need the reprompt gate before anything is
// copied, which this component does not provide.
export function CopyButton({ text, tooltip }: { text: string; tooltip: string }) {
  return (
    <Tooltip text={tooltip}>
      <button
        type="button"
        onClick={() => {
          api.copyToClipboard(text);
          toast("Copied. It will clear from the clipboard in 30s.");
        }}
        className="rounded px-2 py-0.5 border border-[var(--border)]"
      >
        <Icon name="clipboard" />
      </button>
    </Tooltip>
  );
}
