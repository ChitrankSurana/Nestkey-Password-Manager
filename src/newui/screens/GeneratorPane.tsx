import { Generator } from "../../screens/Generator";

// One-to-one passthrough to the existing Generator screen (D-11) - it already renders
// `flex-1 p-4` internally, so it drops straight into NewShell's detail column with no added
// layout of its own.
export function GeneratorPane({ onBack }: { onBack: () => void }) {
  return <Generator onBack={onBack} />;
}
