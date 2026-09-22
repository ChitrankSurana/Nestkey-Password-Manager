import { SecretField } from "../../components/SecretField";
import { CopyButton } from "./CopyButton";
import type { CustomFieldType } from "../../lib/api";

// One labelled detail row. Mirrors src/newui/components/DetailField.tsx's branching exactly
// (hidden -> SecretField carrying the same reprompt flag, boolean -> Yes/No, text -> mono +
// copy) - restyled onto the mk-* tokens per B3-7 (restyle the real component, never reimplement).
export function DetailField({
  label,
  value,
  fieldType,
  reprompt,
  copyable = true,
}: {
  label: string;
  value: string;
  fieldType: CustomFieldType;
  reprompt: boolean;
  copyable?: boolean;
}) {
  return (
    <div className="group">
      <p className="text-[10px] font-medium uppercase tracking-widest mb-1 text-mk-faint">{label}</p>
      <div className="flex items-center gap-2">
        {fieldType === "hidden" ? (
          <SecretField secret={value} reprompt={reprompt} />
        ) : fieldType === "boolean" ? (
          <span className="flex-1 text-sm text-mk-fg2">{value === "true" ? "Yes" : "No"}</span>
        ) : (
          <>
            <span className="flex-1 text-sm break-all leading-relaxed select-all text-mk-fg2 font-mk-mono">{value}</span>
            {copyable && <CopyButton text={value} title={`Copy ${label}`} />}
          </>
        )}
      </div>
    </div>
  );
}
