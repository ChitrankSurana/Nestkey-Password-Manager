import { Trash } from "../../screens/Trash";

// One-to-one passthrough to the existing Trash screen (D-11) - it already renders `flex-1 p-4`
// internally, so it drops straight into NewShell's detail column with no added layout of its own.
export function TrashPane({ onBack }: { onBack: () => void }) {
  return <Trash onBack={onBack} />;
}
