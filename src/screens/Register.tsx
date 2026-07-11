import { useState, FormEvent } from "react";
import { api, isCommandError } from "../lib/api";
import { Tooltip } from "../components/Tooltip";

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
    <div className="min-h-screen flex items-center justify-center dark:bg-slate-900 dark:text-slate-100">
      <form onSubmit={handleSubmit} className="w-80 space-y-4">
        <h1 className="text-xl font-semibold">Register</h1>
        <label className="block">
          <span className="text-sm">Choose a username</span>
          <input
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            autoFocus
            autoComplete="username"
            title="A unique username for this vault"
          />
        </label>
        <label className="block">
          <span className="text-sm">Choose a master password</span>
          <input
            type="password"
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="new-password"
            title="This encrypts your vault - there is no recovery if you forget it"
          />
        </label>
        {error && <p className="text-red-400 text-sm">{error}</p>}
        <Tooltip text="Create your account and vault" className="block w-full">
          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded bg-blue-600 py-1.5 disabled:opacity-50"
          >
            Register
          </button>
        </Tooltip>
        <Tooltip text="Already have an account? Log in" className="block w-full">
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
