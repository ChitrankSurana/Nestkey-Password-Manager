import { useState } from "react";
import { Backup } from "./Backup";
import { CsvExport } from "./CsvExport";
import { CsvImport } from "./CsvImport";
import { JsonExport } from "./JsonExport";
import { JsonImport } from "./JsonImport";
import { IconDatabase, IconDownload, IconPlus } from "../icons";

type DataScreen = "hub" | "backup" | "export" | "import" | "jsonExport" | "jsonImport";

const ACTIONS: { id: DataScreen; Icon: typeof IconDatabase; title: string; desc: string }[] = [
  { id: "backup", Icon: IconDownload, title: "Backup & Restore", desc: "Create or restore an encrypted vault backup" },
  { id: "jsonExport", Icon: IconDatabase, title: "Export JSON (full fidelity)", desc: "Everything: folders, TOTP, custom fields, history" },
  { id: "jsonImport", Icon: IconPlus, title: "Import JSON (full fidelity)", desc: "Merge a full-fidelity export into this vault" },
  { id: "export", Icon: IconDatabase, title: "Export CSV", desc: "Encrypted or confirmed-plaintext export" },
  { id: "import", Icon: IconPlus, title: "Import CSV", desc: "From Chrome/Firefox export or a mapped CSV" },
];

// Restyled chrome onto mk-* tokens, matching figma-make/src/App.tsx:1303-1331's ScreenHeader +
// action-tile grid shape. Three tiles (not Figma's four) since the real product combines
// backup+restore into one screen (src/screens/Data.tsx) - restyle follows the real flow, not the
// mockup's split. No onBack on the hub itself, matching TrashView/HealthDashboard/Generator; the
// sub-screens keep their own "back to hub" link since that's in-hub navigation, not vault nav.
export function DataHub() {
  const [screen, setScreen] = useState<DataScreen>("hub");

  if (screen === "backup") return <Backup onBack={() => setScreen("hub")} />;
  if (screen === "export") return <CsvExport onBack={() => setScreen("hub")} />;
  if (screen === "import") return <CsvImport onBack={() => setScreen("hub")} />;
  if (screen === "jsonExport") return <JsonExport onBack={() => setScreen("hub")} />;
  if (screen === "jsonImport") return <JsonImport onBack={() => setScreen("hub")} />;

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-mk-base">
      <div className="h-14 px-6 flex items-center gap-4 flex-shrink-0 border-b border-mk-border">
        <div className="w-9 h-9 rounded-xl flex items-center justify-center flex-shrink-0 bg-mk-accent-muted">
          <IconDatabase size={18} className="text-mk-accent" />
        </div>
        <div>
          <h1 className="text-sm font-bold text-mk-fg">Data Hub</h1>
          <p className="text-xs text-mk-muted">Backup, export, and import your vault</p>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-2xl mx-auto grid gap-3 sm:grid-cols-2">
          {ACTIONS.map(({ id, Icon, title, desc }) => (
            <button
              key={id}
              onClick={() => setScreen(id)}
              className="p-4 rounded-xl border border-mk-border bg-mk-card hover:bg-mk-hover text-left flex items-start gap-3 transition-colors"
            >
              <div className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5 bg-mk-accent-muted text-mk-accent">
                <Icon size={15} />
              </div>
              <div>
                <p className="text-sm font-semibold text-mk-fg">{title}</p>
                <p className="text-xs mt-0.5 text-mk-muted">{desc}</p>
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
