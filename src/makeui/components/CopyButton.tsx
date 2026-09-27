import { api } from "../../lib/api";
import { toast } from "../../lib/toast";
import { IconCopy } from "../icons";

// Non-secret copy control, routed through the one clipboard seam (mirrors
// src/newui/components/CopyButton.tsx exactly; SecretField owns secret copies).
// N9: autoClear=false and a plain "Copied." toast - the 30s auto-clear and its notice previously
// applied here too, but that behavior belongs to secrets (passwords), not a username or URL.
export function CopyButton({ text, title = "Copy" }: { text: string; title?: string }) {
  return (
    <button
      type="button"
      title={title}
      onClick={() => {
        api
          .copyToClipboard(text, false)
          .then(() => toast("Copied."))
          .catch(() => toast("Couldn't copy to the clipboard. Please try again."));
      }}
      className="flex items-center justify-center w-7 h-7 rounded-md text-mk-muted bg-mk-badge hover:opacity-80 transition-opacity"
    >
      <IconCopy size={12} />
    </button>
  );
}
