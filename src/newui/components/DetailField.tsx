import { SecretField } from "../../components/SecretField";
import { CopyButton } from "./CopyButton";
import type { CustomFieldType } from "../../lib/api";

// One labelled detail row, branching exactly as SiteDetail.tsx already branches (lines 108-114):
// a hidden field delegates to SecretField carrying the same reprompt flag the main secret uses -
// routing it anywhere else would silently drop the reprompt gate on exactly the fields most
// likely to hold a second secret (CONTEXT.md flags forgetting this as the easy mistake). A
// boolean field renders the word Yes/No from the string comparison against "true". A text field
// renders the raw value in font-mono with break-all so a long key wraps inside the pane instead
// of overflowing it.
export function DetailField({
  label,
  value,
  fieldType,
  reprompt,
  copyable,
}: {
  label: string;
  value: string;
  fieldType: CustomFieldType;
  reprompt: boolean;
  copyable?: boolean;
}) {
  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="w-32 shrink-0 truncate text-xs font-semibold uppercase tracking-widest opacity-70">
        {label}
      </span>
      {fieldType === "hidden" ? (
        <SecretField secret={value} reprompt={reprompt} />
      ) : fieldType === "boolean" ? (
        <span className="flex-1">{value === "true" ? "Yes" : "No"}</span>
      ) : (
        <>
          <span className="flex-1 break-all font-mono">{value}</span>
          {copyable && <CopyButton text={value} tooltip={`Copy ${label}`} />}
        </>
      )}
    </div>
  );
}
