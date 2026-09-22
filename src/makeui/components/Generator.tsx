import { useEffect, useState } from "react";
import { api } from "../../lib/api";
import { PasswordGeneratorPanel } from "../../components/PasswordGeneratorPanel";
import { IconZap } from "../icons";

// Restyled chrome onto mk-* tokens, matching figma-make/src/App.tsx:1121-1300's ScreenHeader +
// output-box shape. PasswordGeneratorPanel itself stays untouched and unstyled inside this chrome
// (B3-7): it's the real generator (random/memorable/pattern/pronounceable methods, real
// api.generatePassword + api.copyToClipboard, zxcvbn/entropy strength meter), already embedded the
// same way in ItemDrawer.tsx - reimplementing its logic in mk-* style would mean two generators to
// keep in sync. No onBack, matching TrashView/HealthDashboard's section-view precedent.
export function Generator() {
  const [secret, setSecret] = useState("");
  const [algorithm, setAlgorithm] = useState("simple");

  useEffect(() => {
    api
      .getConfig()
      .then((c) => setAlgorithm(c.strengthAlgorithm))
      .catch(() => {});
  }, []);

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-mk-base">
      <div className="h-14 px-6 flex items-center gap-4 flex-shrink-0 border-b border-mk-border">
        <div className="w-9 h-9 rounded-xl flex items-center justify-center flex-shrink-0 bg-mk-accent-muted">
          <IconZap size={18} className="text-mk-accent" />
        </div>
        <div>
          <h1 className="text-sm font-bold text-mk-fg">Password Generator</h1>
          <p className="text-xs text-mk-muted">Generate strong, secure passwords</p>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-2xl mx-auto space-y-4">
          <div className="rounded-xl border border-mk-border bg-mk-card px-5 py-4">
            <label className="block">
              <span className="text-xs font-medium text-mk-muted">Generated password</span>
              <input
                readOnly
                className="mt-2 w-full rounded-lg border px-3 py-2 bg-mk-input border-mk-border text-mk-fg font-mk-mono text-sm"
                value={secret}
                placeholder="Click Generate below"
              />
            </label>
          </div>

          <PasswordGeneratorPanel value={secret} onChange={setSecret} algorithm={algorithm} />
        </div>
      </div>
    </div>
  );
}
