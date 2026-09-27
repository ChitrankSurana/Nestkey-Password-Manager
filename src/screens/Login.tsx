import { useState, FormEvent } from "react";
import { api, isCommandError, isTauri } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { Spinner } from "../components/Spinner";

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
  const [biometricAvailable, setBiometricAvailable] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function checkQuickUnlockAvailable() {
    if (username.trim() === "") return;
    try {
      setPinAvailable(await api.hasPin(username));
    } catch {
      setPinAvailable(false);
    }
    if (isTauri) {
      try {
        setBiometricAvailable(await api.hasBiometric(username));
      } catch {
        setBiometricAvailable(false);
      }
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
      // B8: the shared backend message ("Incorrect master password.") is correct wording for a
      // reauth prompt (this user's own password, no username field), but on the Login screen it
      // names the wrong field when the actual problem could equally be the username - reword only
      // here, not in core/src/error.rs, since that message is also used by reauth prompts where the
      // more specific wording is accurate.
      if (isCommandError(err) && err.code === "invalid_credentials") {
        setError("Invalid username or password.");
      } else {
        setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      }
    } finally {
      setPassword("");
      setPin("");
      setSubmitting(false);
    }
  }

  async function handleBiometricUnlock() {
    setError(null);
    setSubmitting(true);
    try {
      const result = await api.unlockWithBiometric(username);
      onSuccess(result.username);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
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
            onBlur={checkQuickUnlockAvailable}
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
        {biometricAvailable && (
          <Tooltip text="Prompts Windows Hello to unlock" className="w-full">
            <button
              type="button"
              onClick={handleBiometricUnlock}
              disabled={submitting}
              className="w-full rounded border py-1.5 border-[var(--border)] disabled:opacity-50"
            >
              {submitting ? (
                <>
                  <Spinner className="mr-1.5 inline-block" />
                  Unlocking…
                </>
              ) : (
                "Unlock with Windows Hello"
              )}
            </button>
          </Tooltip>
        )}
        {error && <p className="text-red-400 text-sm">{error}</p>}
        <Tooltip text="Log in to your vault" className="w-full">
          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded bg-[var(--accent)] py-1.5 disabled:opacity-50"
          >
            {submitting ? (
              <>
                <Spinner className="mr-1.5 inline-block" />
                Logging in…
              </>
            ) : (
              "Log in"
            )}
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
