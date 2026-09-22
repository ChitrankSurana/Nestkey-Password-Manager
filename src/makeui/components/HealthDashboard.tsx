import { useEffect, useState } from "react";
import { api, isCommandError, type HealthReportDto, type HealthEntryDto, type BreachReportDto } from "../../lib/api";
import { IconActivity, IconChevron, IconCheck, IconKey } from "../icons";

// Restyled onto mk-* tokens from figma-make/src/App.tsx:946-1096 (ScreenHeader + category cards),
// same "real data, real behavior" rule as DetailPanel/Sidebar/ItemList in steps 6-9 - not a
// reimplementation of src/screens/HealthDashboard.tsx's logic. No onBack: this is a section view
// like TrashView, navigated away via Sidebar, not a full-width page with its own back button
// (Figma's ScreenHeader has none either). The prototype's "No 2FA" category has no backend
// equivalent (HealthReportDto has weak/reused/old/unsecured, not totp-related) so it's dropped;
// "Old" keeps Figma's muted-grey treatment (--mk-text-muted) since the real report doesn't fold it
// into the same warning bucket as reused/unsecured.
type CategoryId = "weak" | "reused" | "old" | "unsecured";

const CATEGORY_META: Record<CategoryId, { label: string; colorVar: string }> = {
  weak: { label: "Weak Passwords", colorVar: "--mk-red" },
  reused: { label: "Reused Passwords", colorVar: "--mk-yellow" },
  old: { label: "Old Passwords", colorVar: "--mk-text-muted" },
  unsecured: { label: "Unsecured Site (http://)", colorVar: "--mk-orange" },
};

function EntryRow({ entry, onNavigateToSite }: { entry: HealthEntryDto; onNavigateToSite: (siteId: number) => void }) {
  return (
    <div className="flex items-center gap-3 px-5 py-3 border-t border-mk-border">
      <div className="w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0 bg-mk-login-bg">
        <IconKey size={13} className="text-mk-login-fg" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-mk-fg2 truncate">{entry.siteName}</p>
        <p className="text-xs text-mk-faint truncate">{entry.username ?? "(no username)"}</p>
      </div>
      <button
        onClick={() => onNavigateToSite(entry.siteId)}
        className="text-xs px-2.5 py-1 rounded-lg font-medium bg-mk-badge text-mk-muted hover:bg-mk-badge-active flex-shrink-0"
      >
        View
      </button>
    </div>
  );
}

export function HealthDashboard({ onNavigateToSite }: { onNavigateToSite: (siteId: number) => void }) {
  const [report, setReport] = useState<HealthReportDto | null>(null);
  const [breach, setBreach] = useState<BreachReportDto | null>(null);
  const [hibpOptIn, setHibpOptIn] = useState(false);
  const [expanded, setExpanded] = useState<CategoryId | null>(null);
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

  const categories: { id: CategoryId; entries: HealthEntryDto[] }[] = report
    ? [
        { id: "weak", entries: report.weak },
        { id: "reused", entries: report.reused.flatMap((g) => g.entries) },
        { id: "old", entries: report.old },
        { id: "unsecured", entries: report.unsecured },
      ]
    : [];
  const totalIssues = categories.reduce((s, c) => s + c.entries.length, 0);

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-mk-base">
      <div className="h-14 px-6 flex items-center gap-4 flex-shrink-0 border-b border-mk-border">
        <div className="w-9 h-9 rounded-xl flex items-center justify-center flex-shrink-0 bg-mk-accent-muted">
          <IconActivity size={18} className="text-mk-accent" />
        </div>
        <div>
          <h1 className="text-sm font-bold text-mk-fg">Health Dashboard</h1>
          <p className="text-xs text-mk-muted">Security analysis of your vault</p>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-3xl mx-auto space-y-4">
          {error && <p className="text-sm text-mk-red">{error}</p>}
          {!report && !error && <p className="text-sm text-mk-faint">Loading…</p>}

          {report && (
            <>
              <p className="text-xs text-mk-faint">
                "Weak" is always scored with the advanced algorithm, regardless of your strength-algorithm setting in Settings.
              </p>

              <div className="flex items-center justify-between">
                <p className="text-xs text-mk-muted">
                  {totalIssues === 0 ? "Your vault looks healthy." : `${totalIssues} issue${totalIssues !== 1 ? "s" : ""} found across your logins.`}
                </p>
                <div className="flex items-center gap-1.5">
                  <div className="w-2 h-2 rounded-full" style={{ background: totalIssues === 0 ? "var(--mk-green)" : "var(--mk-yellow)" }} />
                  <span className="text-xs font-medium" style={{ color: totalIssues === 0 ? "var(--mk-green)" : "var(--mk-yellow)" }}>
                    {totalIssues === 0 ? "All clear" : "Attention needed"}
                  </span>
                </div>
              </div>

              {categories.map((cat) => {
                const meta = CATEGORY_META[cat.id];
                const isOpen = expanded === cat.id;
                return (
                  <div key={cat.id} className="rounded-xl border border-mk-border bg-mk-card overflow-hidden">
                    <button
                      onClick={() => setExpanded(isOpen ? null : cat.id)}
                      className="w-full flex items-center gap-4 px-5 py-4 hover:bg-mk-hover transition-colors"
                    >
                      <div
                        className="w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0"
                        style={{ background: `color-mix(in srgb, var(${meta.colorVar}) 12%, transparent)` }}
                      >
                        <span className="text-base font-bold" style={{ color: `var(${meta.colorVar})` }}>
                          {cat.entries.length}
                        </span>
                      </div>
                      <div className="flex-1 text-left">
                        <p className="text-sm font-medium text-mk-fg">{meta.label}</p>
                        <p className="text-xs text-mk-muted">
                          {cat.entries.length === 0 ? "No issues found" : `${cat.entries.length} login${cat.entries.length !== 1 ? "s" : ""} affected`}
                        </p>
                      </div>
                      {cat.entries.length > 0 ? (
                        <IconChevron
                          size={14}
                          className="text-mk-faint transition-transform duration-200"
                          style={{ transform: isOpen ? "rotate(90deg)" : undefined }}
                        />
                      ) : (
                        <IconCheck size={14} style={{ color: "var(--mk-green)" }} />
                      )}
                    </button>
                    {isOpen && cat.entries.length > 0 && (
                      <div>
                        {cat.entries.map((e) => (
                          <EntryRow key={e.credentialId} entry={e} onNavigateToSite={onNavigateToSite} />
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}

              <div className="rounded-xl border border-mk-border bg-mk-card overflow-hidden">
                <div className="flex items-center gap-4 px-5 py-4">
                  <div
                    className="w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0"
                    style={{ background: "color-mix(in srgb, var(--mk-red) 12%, transparent)" }}
                  >
                    <span className="text-base font-bold" style={{ color: "var(--mk-red)" }}>
                      {breach ? breach.breached.length : "—"}
                    </span>
                  </div>
                  <div className="flex-1">
                    <p className="text-sm font-medium text-mk-fg">Breached</p>
                    <p className="text-xs text-mk-muted">
                      {hibpOptIn
                        ? breach
                          ? `${breach.breached.length} exposed login${breach.breached.length !== 1 ? "s" : ""} found`
                          : "Not checked yet"
                        : 'Enable "Breach check (HIBP)" in Settings first'}
                    </p>
                  </div>
                  {hibpOptIn && (
                    <button
                      onClick={handleCheckBreach}
                      disabled={checkingBreach}
                      className="text-xs px-3 py-1.5 rounded-lg font-medium bg-mk-accent text-white disabled:opacity-50 flex-shrink-0"
                    >
                      {checkingBreach ? "Checking…" : "Check now"}
                    </button>
                  )}
                </div>
                {breach && breach.breached.length > 0 && (
                  <div>
                    {breach.breached.map((e) => (
                      <EntryRow key={e.credentialId} entry={e} onNavigateToSite={onNavigateToSite} />
                    ))}
                  </div>
                )}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
