import { useState, FormEvent } from "react";
import { api, isCommandError } from "../lib/api";
import { Tooltip } from "../components/Tooltip";
import { Icon } from "../components/Icon";
import { RequiredLegend, RequiredMark } from "../components/Required";
import { Spinner } from "../components/Spinner";

export function Register({
  onSuccess,
  onSwitchToLogin,
}: {
  onSuccess: (username: string) => void;
  onSwitchToLogin: () => void;
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
      const result = await api.registerUser(username, password);
      onSuccess(result.username);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setPassword("");
      setSubmitting(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-[var(--bg)] text-[var(--fg)]">
      <form onSubmit={handleSubmit} className="w-80 space-y-4">
        <h1 className="text-xl font-semibold">Register</h1>
        <label className="block">
          <span className="flex items-center gap-1 text-sm">
            <span>Choose a username<RequiredMark /></span>
            <Tooltip text="Username is case-sensitive">
              <button type="button" aria-label="Username is case-sensitive" className="opacity-70 hover:opacity-100">
                <Icon name="info-circle" />
              </button>
            </Tooltip>
          </span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            required
            autoFocus
            autoComplete="username"
            title="A unique username for this vault"
          />
        </label>
        <label className="block">
          <span className="text-sm">Choose a master password<RequiredMark /></span>
          <input
            type="password"
            className="mt-1 w-full rounded border px-2 py-1 bg-[var(--surface)] border-[var(--border)]"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
            autoComplete="new-password"
            title="This encrypts your vault - there is no recovery if you forget it"
          />
        </label>
        {/* N1: was a `title=` tooltip attribute only (hover-only, invisible until the user
            happens to hover the password field) - real visible warning copy instead. */}
        <p className="text-xs text-yellow-500">
          There is no password recovery. If you forget this master password, your vault cannot be
          decrypted by anyone, including you.
        </p>
        {error && <p className="text-red-400 text-sm">{error}</p>}
        <RequiredLegend />
        <Tooltip text="Create your account and vault" className="w-full">
          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded bg-[var(--accent)] py-1.5 disabled:opacity-50"
          >
            {submitting ? (
              <>
                <Spinner className="mr-1.5 inline-block" />
                Creating your vault…
              </>
            ) : (
              "Register"
            )}
          </button>
        </Tooltip>
        <Tooltip text="Already have an account? Log in" className="w-full">
          <button
            type="button"
            onClick={onSwitchToLogin}
            className="w-full text-sm underline"
          >
            Already have an account? Log in
          </button>
        </Tooltip>
      </form>
    </div>
  );
}
