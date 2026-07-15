// src/screens/HealthDashboard.tsx
import { useEffect, useState } from "react";
import { api, isCommandError, HealthReportDto, HealthEntryDto, BreachReportDto } from "../lib/api";
import { Tooltip } from "../components/Tooltip";

function EntryList({ entries, onNavigateToSite }: { entries: HealthEntryDto[]; onNavigateToSite: (siteId: number) => void }) {
  return (
    <div className="mt-2 space-y-1">
      {entries.map((e) => (
        <Tooltip key={e.credentialId} text="Go to this site to edit the credential">
          <button
            onClick={() => onNavigateToSite(e.siteId)}
            className="block w-full rounded px-2 py-1 text-left text-sm hover:bg-slate-800"
          >
            {e.siteName} — {e.username ?? "(no username)"}
          </button>
        </Tooltip>
      ))}
      {entries.length === 0 && <p className="text-sm opacity-60">None.</p>}
    </div>
  );
}

export function HealthDashboard({
  onBack,
  onNavigateToSite,
}: {
  onBack: () => void;
  onNavigateToSite: (siteId: number) => void;
}) {
  const [report, setReport] = useState<HealthReportDto | null>(null);
  const [breach, setBreach] = useState<BreachReportDto | null>(null);
  const [hibpOptIn, setHibpOptIn] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [checkingBreach, setCheckingBreach] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        setError(null);
        const [r, c] = await Promise.all([api.healthReport(), api.getConfig()]);
        setReport(r);
        setHibpOptIn(c.hibpOptIn);
      } catch (err) {
        setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      }
    })();
  }, []);

  async function handleCheckBreach() {
    setCheckingBreach(true);
    try {
      setError(null);
      setBreach(await api.checkBreach());
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setCheckingBreach(false);
    }
  }

  function toggle(key: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  return (
    <div className="min-h-screen dark:bg-slate-900 dark:text-slate-100 p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 underline text-sm">← Back</button>
      </Tooltip>
      <h1 className="text-xl font-semibold mb-4">Health Dashboard</h1>
      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}
      {!report ? (
        <p className="text-sm opacity-70">Loading…</p>
      ) : (
        <>
          <p className="text-xs opacity-60 mb-4">
            "Weak" is always scored with the advanced algorithm, regardless of your strength-algorithm
            setting in Settings.
          </p>
          <div className="space-y-3 max-w-2xl">
            <div className="rounded border p-3 dark:border-slate-700">
              <Tooltip text="Show or hide the list">
                <button onClick={() => toggle("weak")} className="flex w-full justify-between items-center">
                  <span className="font-medium">⚠️ Weak</span>
                  <span>{report.weak.length}</span>
                </button>
              </Tooltip>
              {expanded.has("weak") && <EntryList entries={report.weak} onNavigateToSite={onNavigateToSite} />}
            </div>

            <div className="rounded border p-3 dark:border-slate-700">
              <Tooltip text="Show or hide the list">
                <button onClick={() => toggle("reused")} className="flex w-full justify-between items-center">
                  <span className="font-medium">♻️ Reused</span>
                  <span>{report.reused.length}</span>
                </button>
              </Tooltip>
              {expanded.has("reused") && (
                <div className="mt-2 space-y-3">
                  {report.reused.map((group, i) => (
                    <EntryList key={i} entries={group.entries} onNavigateToSite={onNavigateToSite} />
                  ))}
                  {report.reused.length === 0 && <p className="text-sm opacity-60">None.</p>}
                </div>
              )}
            </div>

            <div className="rounded border p-3 dark:border-slate-700">
              <Tooltip text="Show or hide the list">
                <button onClick={() => toggle("old")} className="flex w-full justify-between items-center">
                  <span className="font-medium">🕰️ Old</span>
                  <span>{report.old.length}</span>
                </button>
              </Tooltip>
              {expanded.has("old") && <EntryList entries={report.old} onNavigateToSite={onNavigateToSite} />}
            </div>

            <div className="rounded border p-3 dark:border-slate-700">
              <div className="flex w-full justify-between items-center">
                <span className="font-medium">🛡️ Breached</span>
                {hibpOptIn ? (
                  <Tooltip text="Check your passwords against known breaches (HIBP)">
                    <button
                      onClick={handleCheckBreach}
                      disabled={checkingBreach}
                      className="rounded bg-blue-600 px-3 py-1 text-sm disabled:opacity-50"
                    >
                      {checkingBreach ? "Checking…" : "Check now"}
                    </button>
                  </Tooltip>
                ) : (
                  <span className="text-xs opacity-60">Enable "Breach check (HIBP)" in Settings first</span>
                )}
              </div>
              {breach && <EntryList entries={breach.breached} onNavigateToSite={onNavigateToSite} />}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
