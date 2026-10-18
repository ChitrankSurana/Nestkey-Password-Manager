// SQLite `datetime('now')` yields "YYYY-MM-DD HH:MM:SS" in UTC with no zone marker.
export function formatUtcLocal(sqliteUtc: string): string {
  const d = new Date(sqliteUtc.replace(" ", "T") + "Z");
  if (Number.isNaN(d.getTime())) return sqliteUtc;
  return d.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}
