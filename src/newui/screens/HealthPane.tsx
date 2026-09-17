import { HealthDashboard } from "../../screens/HealthDashboard";

// One-to-one passthrough to the existing HealthDashboard screen (D-11) - it already renders
// `flex-1 p-4` internally, so it drops straight into NewShell's detail column with no added
// layout of its own. Threads both onBack and onNavigateToSite through unchanged.
export function HealthPane({
  onBack,
  onNavigateToSite,
}: {
  onBack: () => void;
  onNavigateToSite: (siteId: number) => void;
}) {
  return <HealthDashboard onBack={onBack} onNavigateToSite={onNavigateToSite} />;
}
