import { Settings } from "../../screens/Settings";

// One-to-one passthrough to the existing Settings screen (D-11) - it already renders
// `flex-1 p-4` internally, so it drops straight into NewShell's detail column with no added
// layout of its own. Settings already gates its desktop-only controls (biometric, SSH agent) on
// the exported isTauri flag internally - reusing it unchanged is what keeps the web portal from
// showing controls it cannot support.
export function SettingsPane({ onBack, username }: { onBack: () => void; username: string }) {
  return <Settings onBack={onBack} username={username} />;
}
