import { api } from "../../lib/api";
import { toast } from "../../lib/toast";
import { Tooltip } from "../../components/Tooltip";
import { Icon } from "../../components/Icon";

// Tooltip-wrapped copy control for non-secret values, routed through the one clipboard seam.
// Secrets are SecretField's job, not this component's - they need the reprompt gate before
// anything is copied, which this component does not provide.
// N9: autoClear=false and a plain "Copied." toast - the 30s auto-clear and its notice previously
// applied here too, but that behavior belongs to secrets (passwords), not a username or URL.
export function CopyButton({ text, tooltip }: { text: string; tooltip: string }) {
  return (
    <Tooltip text={tooltip}>
      <button
        type="button"
        onClick={() => {
          api
            .copyToClipboard(text, false)
            .then(() => toast("Copied."))
            .catch(() => toast("Couldn't copy to the clipboard. Please try again."));
        }}
        className="rounded px-2 py-0.5 border border-[var(--border)]"
      >
        <Icon name="clipboard" />
      </button>
    </Tooltip>
  );
}
