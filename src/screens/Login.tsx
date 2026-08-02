import { useState, FormEvent } from "react";
import { api, isCommandError } from "../lib/api";
import { Tooltip } from "../components/Tooltip";

export function Login({
  onSuccess,
  onSwitchToRegister,
}: {
  onSuccess: (username: string) => void;
  onSwitchToRegister?: () => void;
}) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [pin, setPin] = useState("");
  const [pinAvailable, setPinAvailable] = useState(false);
  const [usePin, setUsePin] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function checkPinAvailable() {
    if (username.trim() === "") return;
    try {
      setPinAvailable(await api.hasPin(username));
    } catch {
      setPinAvailable(false);
    }
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const result = usePin ? await api.unlockWithPin(username, pin) : await api.login(username, password);
      onSuccess(result.username);
    } catch (err) {
      // Single generic message on any failure (TRD §3.2 / THREAT_MODEL wrong-password oracle) -
      // do not display err.message variations beyond what the backend already collapsed to one
      // string; just surface it as-is since the backend already guarantees it's generic.
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setPassword("");
      setPin("");
      setSubmitting(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-[var(--bg)] text-[var(--fg)]">
      <form onSubmit={handleSubmit} className="w-80 space-y-4">
        <h1 className="text-xl font-semibold">Log in</h1>
        <label className="block">
          <span className="text-sm">Username</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            onBlur={checkPinAvailable}
            autoFocus
            autoComplete="username"
            title="Your registered username"
          />
        </label>
        {usePin ? (
          <label className="block">
            <span className="text-sm">PIN</span>
            <input
              type="password"
              inputMode="numeric"
              className="mt-1 w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
              value={pin}
              onChange={(e) => setPin(e.target.value)}
              title="The PIN you set up for quick unlock"
            />
          </label>
        ) : (
          <label className="block">
            <span className="text-sm">Master password</span>
            <input
              type="password"
              className="mt-1 w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="current-password"
              title="The master password you registered with"
            />
          </label>
        )}
        {pinAvailable && (
          <button
            type="button"
            onClick={() => setUsePin((v) => !v)}
            className="text-sm underline"
          >
            {usePin ? "Use master password instead" : "Unlock with PIN instead"}
          </button>
        )}
        {error && <p className="text-red-400 text-sm">{error}</p>}
        <Tooltip text="Log in to your vault" className="w-full">
          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded bg-[var(--accent)] py-1.5 disabled:opacity-50"
          >
            Log in
          </button>
        </Tooltip>
        {onSwitchToRegister && (
          <Tooltip text="Create a new account" className="w-full">
            <button
              type="button"
              onClick={onSwitchToRegister}
              className="w-full text-sm underline"
            >
              Need an account? Register
            </button>
          </Tooltip>
        )}
      </form>
    </div>
  );
}
