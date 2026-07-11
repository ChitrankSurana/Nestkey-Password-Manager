import { useState, FormEvent } from "react";
import { api, isCommandError } from "../lib/api";
import { Tooltip } from "../components/Tooltip";

export function Login({
  onSuccess,
  onSwitchToRegister,
}: {
  onSuccess: (username: string) => void;
  onSwitchToRegister: () => void;
}) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const result = await api.login(username, password);
      onSuccess(result.username);
    } catch (err) {
      // Single generic message on any failure (TRD §3.2 / THREAT_MODEL wrong-password oracle) -
      // do not display err.message variations beyond what the backend already collapsed to one
      // string; just surface it as-is since the backend already guarantees it's generic.
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setPassword("");
      setSubmitting(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center dark:bg-slate-900 dark:text-slate-100">
      <form onSubmit={handleSubmit} className="w-80 space-y-4">
        <h1 className="text-xl font-semibold">Log in</h1>
        <label className="block">
          <span className="text-sm">Username</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoFocus
            autoComplete="username"
            title="Your registered username"
          />
        </label>
        <label className="block">
          <span className="text-sm">Master password</span>
          <input
            type="password"
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
            title="The master password you registered with"
          />
        </label>
        {error && <p className="text-red-400 text-sm">{error}</p>}
        <Tooltip text="Log in to your vault" className="w-full">
          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded bg-blue-600 py-1.5 disabled:opacity-50"
          >
            Log in
          </button>
        </Tooltip>
        <Tooltip text="Create a new account" className="w-full">
          <button
            type="button"
            onClick={onSwitchToRegister}
            className="w-full text-sm underline"
          >
            Need an account? Register
          </button>
        </Tooltip>
      </form>
    </div>
  );
}
