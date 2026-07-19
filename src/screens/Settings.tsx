import { useEffect, useRef, useState } from "react";
import { api, isCommandError } from "../lib/api";
import { Tooltip } from "../components/Tooltip";

const OPTIONS = [1, 5, 15] as const;
const ALGORITHM_OPTIONS = [
  { value: "simple", label: "Simple (entropy)" },
  { value: "zxcvbn", label: "Advanced (zxcvbn)" },
] as const;
const ACCENT_OPTIONS = [
  { value: "indigo", label: "Indigo" },
  { value: "sky", label: "Sky" },
  { value: "teal", label: "Teal" },
  { value: "purple", label: "Purple" },
  { value: "pink", label: "Pink" },
] as const;
const MOBILE_NAV_STYLE_OPTIONS = [
  { value: "icon-rail", label: "Icon rail (always visible)" },
  { value: "bottom-tabs", label: "Bottom tab bar" },
  { value: "drawer", label: "Hamburger menu" },
] as const;
const OLD_PASSWORD_DAYS_OPTIONS = [90, 180, 365] as const;

export function Settings({ onBack }: { onBack: () => void }) {
  const [autoLockMinutes, setAutoLockMinutes] = useState<number | null>(null);
  const [strengthAlgorithm, setStrengthAlgorithm] = useState<string | null>(null);
  const [theme, setTheme] = useState<string | null>(null);
  const [accentColor, setAccentColor] = useState<string | null>(null);
  const [oldPasswordDays, setOldPasswordDays] = useState<number | null>(null);
  const [hibpOptIn, setHibpOptIn] = useState<boolean | null>(null);
  const [mobileNavStyle, setMobileNavStyle] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const editedRef = useRef(false);

  useEffect(() => {
    api
      .getConfig()
      .then((c) => {
        if (!editedRef.current) {
          setAutoLockMinutes(c.autoLockMinutes);
          setStrengthAlgorithm(c.strengthAlgorithm);
          setTheme(c.theme);
          setAccentColor(c.accentColor);
          setOldPasswordDays(c.oldPasswordDays);
          setHibpOptIn(c.hibpOptIn);
          setMobileNavStyle(c.mobileNavStyle);
        }
      })
      .catch((err) => {
        setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      });
  }, []);

  async function handleChange(minutes: number) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateAutoLockMinutes(minutes);
      setAutoLockMinutes(minutes);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleAlgorithmChange(algorithm: string) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateStrengthAlgorithm(algorithm);
      setStrengthAlgorithm(algorithm);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleThemeChange(value: string) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateTheme(value);
      document.documentElement.classList.toggle("dark", value === "dark");
      setTheme(value);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleAccentChange(value: string) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateAccentColor(value);
      document.documentElement.setAttribute("data-accent", value);
      setAccentColor(value);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleOldPasswordDaysChange(value: number) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateOldPasswordDays(value);
      setOldPasswordDays(value);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleHibpOptInChange(value: boolean) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateHibpOptIn(value);
      setHibpOptIn(value);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleMobileNavStyleChange(value: string) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateMobileNavStyle(value);
      setMobileNavStyle(value);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="min-h-screen dark:bg-slate-900 dark:text-slate-100 p-4">
      <Tooltip text="Back to vault">
        <button onClick={onBack} className="mb-4 underline text-sm">
          ← Back
        </button>
      </Tooltip>
      <h1 className="text-xl font-semibold mb-4">Settings</h1>

      {error && <p className="text-sm text-red-400 mb-4">{error}</p>}

      <label className="block max-w-xs">
        <span className="text-sm">Auto-lock after idle for</span>
        <Tooltip text="How long to wait before auto-locking the vault" className="w-full">
          <select
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600 disabled:opacity-50"
            value={autoLockMinutes ?? ""}
            onChange={(e) => handleChange(Number(e.target.value))}
            disabled={submitting}
          >
            {OPTIONS.map((m) => (
              <option key={m} value={m}>
                {m} minute{m === 1 ? "" : "s"}
              </option>
            ))}
          </select>
        </Tooltip>
      </label>

      <label className="block max-w-xs mt-4">
        <span className="text-sm">Password strength algorithm</span>
        <Tooltip text="Which algorithm scores password strength" className="w-full">
          <select
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600 disabled:opacity-50"
            value={strengthAlgorithm ?? ""}
            onChange={(e) => handleAlgorithmChange(e.target.value)}
            disabled={submitting}
          >
            {ALGORITHM_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </Tooltip>
      </label>

      <label className="block max-w-xs mt-4">
        <span className="text-sm">Theme</span>
        <Tooltip text="Light or dark appearance" className="w-full">
          <select
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600 disabled:opacity-50"
            value={theme ?? ""}
            onChange={(e) => handleThemeChange(e.target.value)}
            disabled={submitting}
          >
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>
        </Tooltip>
      </label>

      <label className="block max-w-xs mt-4">
        <span className="text-sm">Accent color</span>
        <Tooltip text="Highlight color used throughout the app" className="w-full">
          <select
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600 disabled:opacity-50"
            value={accentColor ?? ""}
            onChange={(e) => handleAccentChange(e.target.value)}
            disabled={submitting}
          >
            {ACCENT_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </Tooltip>
      </label>

      <label className="block max-w-xs mt-4">
        <span className="text-sm">Consider a password "old" after</span>
        <Tooltip text="Threshold used by the Health Dashboard's Old card" className="w-full">
          <select
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600 disabled:opacity-50"
            value={oldPasswordDays ?? ""}
            onChange={(e) => handleOldPasswordDaysChange(Number(e.target.value))}
            disabled={submitting}
          >
            {OLD_PASSWORD_DAYS_OPTIONS.map((d) => (
              <option key={d} value={d}>
                {d} days
              </option>
            ))}
          </select>
        </Tooltip>
      </label>

      <label className="flex items-center gap-2 mt-4">
        <Tooltip text="Opt in to checking passwords against Have I Been Pwned (k-anonymity, no plaintext leaves this app)">
          <input
            type="checkbox"
            checked={hibpOptIn ?? false}
            onChange={(e) => handleHibpOptInChange(e.target.checked)}
            disabled={submitting}
          />
        </Tooltip>
        <span className="text-sm">Enable breach check (HIBP) on the Health Dashboard</span>
      </label>

      <label className="block max-w-xs mt-4">
        <span className="text-sm">Mobile navigation style</span>
        <Tooltip text="How the sidebar is replaced on narrow screens/windows below 640px" className="w-full">
          <select
            className="mt-1 w-full rounded border px-2 py-1 dark:bg-slate-800 dark:border-slate-600 disabled:opacity-50"
            value={mobileNavStyle ?? ""}
            onChange={(e) => handleMobileNavStyleChange(e.target.value)}
            disabled={submitting}
          >
            {MOBILE_NAV_STYLE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </Tooltip>
      </label>
    </div>
  );
}
