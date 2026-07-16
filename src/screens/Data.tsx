import { useState } from "react";
import { Backup } from "./Backup";
import { CsvExport } from "./CsvExport";
import { CsvImport } from "./CsvImport";
import { Tooltip } from "../components/Tooltip";

type DataScreen = "hub" | "backup" | "export" | "import";

export function Data({ onBack }: { onBack: () => void }) {
  const [screen, setScreen] = useState<DataScreen>("hub");

  if (screen === "backup") return <Backup onBack={() => setScreen("hub")} />;
  if (screen === "export") return <CsvExport onBack={() => setScreen("hub")} />;
  if (screen === "import") return <CsvImport onBack={() => setScreen("hub")} />;

  return (
    <div className="min-h-screen dark:bg-slate-900 dark:text-slate-100 p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 underline text-sm">← Back</button>
      </Tooltip>
      <h1 className="text-xl font-semibold mb-4">Data</h1>
      <div className="grid gap-3 max-w-md">
        <Tooltip text="Backup and restore your vault">
          <button
            onClick={() => setScreen("backup")}
            className="rounded border p-3 text-left dark:border-slate-700 dark:bg-slate-800"
          >
            <div className="font-medium">Backup &amp; Restore</div>
            <div className="text-sm opacity-70">Create or restore an encrypted vault backup</div>
          </button>
        </Tooltip>
        <Tooltip text="Export your vault to a CSV file">
          <button
            onClick={() => setScreen("export")}
            className="rounded border p-3 text-left dark:border-slate-700 dark:bg-slate-800"
          >
            <div className="font-medium">Export CSV</div>
            <div className="text-sm opacity-70">Encrypted or confirmed-plaintext export</div>
          </button>
        </Tooltip>
        <Tooltip text="Import credentials from a CSV file">
          <button
            onClick={() => setScreen("import")}
            className="rounded border p-3 text-left dark:border-slate-700 dark:bg-slate-800"
          >
            <div className="font-medium">Import CSV</div>
            <div className="text-sm opacity-70">From Chrome/Firefox export or a mapped CSV</div>
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
