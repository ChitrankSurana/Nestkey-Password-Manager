import { Data } from "../../screens/Data";

// One-to-one passthrough to the existing Data screen (D-11) - it already renders `flex-1 p-4`
// internally, so it drops straight into NewShell's detail column with no added layout of its own.
export function DataPane({ onBack }: { onBack: () => void }) {
  return <Data onBack={onBack} />;
}
