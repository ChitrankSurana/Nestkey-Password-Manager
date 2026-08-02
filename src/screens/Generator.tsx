import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { PasswordGeneratorPanel } from "../components/PasswordGeneratorPanel";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";

export function Generator({ onBack }: { onBack: () => void }) {
  const [secret, setSecret] = useState("");
  const [algorithm, setAlgorithm] = useState("simple");

  useEffect(() => {
    api.getConfig().then((c) => setAlgorithm(c.strengthAlgorithm)).catch(() => {});
  }, []);

  return (
    <div className="flex-1 bg-[var(--bg)] text-[var(--fg)] p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 underline text-sm"><Icon name="arrow-left" /> Back</button>
      </Tooltip>
      <h1 className="text-xl font-semibold mb-4">Generator</h1>
      <div className="max-w-sm space-y-3">
        <label className="block">
          <span className="text-sm">Generated password</span>
          <input
            readOnly
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--bg)] border-[var(--border)]"
            value={secret}
            placeholder="Click Generate below"
          />
        </label>
        <PasswordGeneratorPanel value={secret} onChange={setSecret} algorithm={algorithm} />
      </div>
    </div>
  );
}
