import { useEffect, useRef, useState, type ComponentType, type FormEvent, type ReactNode } from "react";
import { api, isCommandError, isTauri, type RepromptMode, type SshKeyDto } from "../../lib/api";
import { refreshRepromptConfig } from "../../lib/reprompt";
import { getRememberLastUsername, setRememberLastUsername as setRememberLastUsernameStored } from "../../lib/lastUsername";
import { MasterPasswordPrompt } from "../../components/MasterPasswordPrompt";
import { IconChevron, IconDatabase, IconLock, IconMonitor, IconPalette, IconTerminal, IconTrash, type IconProps } from "../icons";

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
const UI_VARIANT_OPTIONS = [
  { value: "classic", label: "Classic" },
  { value: "new", label: "New" },
  { value: "make", label: "Make" },
] as const;
const OLD_PASSWORD_DAYS_OPTIONS = [90, 180, 365] as const;
// B7: every credential is gated by the master password by default now (a per-credential checkbox
// can opt one out) - this only picks how the shared grace window behaves.
const REPROMPT_MODE_OPTIONS = [
  { value: "grace", label: "Grace window (default)" },
  { value: "always", label: "Always (no grace window)" },
] as const;
// Must match the Rust allowlist (VALID_REPROMPT_GRACE_SECONDS, core/src/config.rs) exactly - a
// value here that the backend rejects would surface as a generic error with no explanation.
const REPROMPT_GRACE_OPTIONS = [15, 30, 60, 300] as const;

const SELECT_CLS = "text-xs rounded-lg border px-2.5 py-1.5 bg-mk-input border-mk-border text-mk-fg2 disabled:opacity-50";
const INPUT_CLS = "w-full rounded-lg border px-3 py-1.5 bg-mk-input border-mk-border text-mk-fg text-sm disabled:opacity-50";
const BTN_PRIMARY_CLS = "rounded-lg bg-mk-accent hover:bg-mk-accent-hover px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50";
const BTN_SECONDARY_CLS = "rounded-lg bg-mk-badge hover:bg-mk-badge-active px-2.5 py-1 text-xs font-medium text-mk-fg2 disabled:opacity-50";

// Accordion chrome ported from figma-make/src/App.tsx:1511-1715 (SettingsToggle + section/field
// helpers) - native <details>/<summary> kept from src/screens/Settings.tsx instead of Figma's
// controlled single-open-section state (same disclosure behavior, zero extra state).
function Section({
  icon: Icon,
  title,
  defaultOpen = false,
  children,
}: {
  icon: ComponentType<IconProps>;
  title: string;
  defaultOpen?: boolean;
  children: ReactNode;
}) {
  return (
    <details open={defaultOpen} className="group rounded-xl border border-mk-border bg-mk-card overflow-hidden">
      <summary className="cursor-pointer list-none marker:hidden w-full flex items-center gap-3 px-5 py-4 hover:bg-mk-hover transition-colors">
        <div className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 bg-mk-accent-muted text-mk-accent">
          <Icon size={15} />
        </div>
        <span className="flex-1 text-left text-sm font-semibold text-mk-fg">{title}</span>
        <IconChevron size={14} className="text-mk-faint transition-transform duration-200 group-open:rotate-90" />
      </summary>
      <div className="px-5 pb-5 pt-4 space-y-5 border-t border-mk-border">{children}</div>
    </details>
  );
}

function Field({ label, desc, children }: { label: string; desc?: string; children: ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-mk-fg">{label}</p>
        {desc && <p className="text-xs mt-0.5 text-mk-muted">{desc}</p>}
      </div>
      <div className="flex-shrink-0">{children}</div>
    </div>
  );
}

function Divider() {
  return <div className="border-t border-mk-border" />;
}

function Toggle({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      disabled={disabled}
      className="relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0 disabled:opacity-50"
      style={{ background: checked ? "var(--accent)" : "var(--mk-badge)" }}
    >
      <span
        className="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200"
        style={{ transform: checked ? "translateX(16px)" : "translateX(0)" }}
      />
    </button>
  );
}

// Restyled chrome onto mk-* tokens; all handlers/state/logic unchanged from src/screens/Settings.tsx
// (re-auth gates via requireMasterPassword for PIN/biometric/SSH-key removal are untouched - B3-7).
// No onBack, matching TrashView/HealthDashboard/Generator/DataHub.
export function Settings({ username }: { username: string }) {
  const [autoLockMinutes, setAutoLockMinutes] = useState<number | null>(null);
  const [strengthAlgorithm, setStrengthAlgorithm] = useState<string | null>(null);
  const [theme, setTheme] = useState<string | null>(null);
  const [accentColor, setAccentColor] = useState<string | null>(null);
  const [oldPasswordDays, setOldPasswordDays] = useState<number | null>(null);
  const [hibpOptIn, setHibpOptIn] = useState<boolean | null>(null);
  const [mobileNavStyle, setMobileNavStyle] = useState<string | null>(null);
  const [uiVariant, setUiVariant] = useState<string | null>(null);
  const bootUiVariantRef = useRef<string | null>(null);
  const [repromptMode, setRepromptMode] = useState<RepromptMode | null>(null);
  const [repromptGraceSeconds, setRepromptGraceSeconds] = useState<number | null>(null);
  const [pinSet, setPinSet] = useState<boolean | null>(null);
  const [pinInput, setPinInput] = useState("");
  const [pwPrompt, setPwPrompt] = useState<{ title: string; resolve: (ok: boolean) => void } | null>(null);
  const [biometricEnabled, setBiometricEnabled] = useState<boolean | null>(null);
  const [biometricSet, setBiometricSet] = useState<boolean | null>(null);
  const [sshAgentEnabled, setSshAgentEnabled] = useState<boolean | null>(null);
  const [sshKeys, setSshKeys] = useState<SshKeyDto[] | null>(null);
  const [sshLabelInput, setSshLabelInput] = useState("");
  const [sshKeyInput, setSshKeyInput] = useState("");
  const [rememberLastUsername, setRememberLastUsername] = useState(() => getRememberLastUsername());
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const editedRef = useRef(false);
  const [currentPwInput, setCurrentPwInput] = useState("");
  const [newPwInput, setNewPwInput] = useState("");
  const [confirmPwInput, setConfirmPwInput] = useState("");
  const [changingPassword, setChangingPassword] = useState(false);
  const [passwordChanged, setPasswordChanged] = useState(false);
  const [pwChangeError, setPwChangeError] = useState<string | null>(null);

  function refreshSshKeys() {
    if (isTauri) {
      api.listSshKeys().then(setSshKeys).catch(() => setSshKeys([]));
    }
  }

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
          setBiometricEnabled(c.biometricEnabled);
          setSshAgentEnabled(c.sshAgentEnabled);
          setUiVariant(c.uiVariant);
          setRepromptMode(c.repromptMode);
          setRepromptGraceSeconds(c.repromptGraceSeconds);
        }
        if (bootUiVariantRef.current === null) {
          bootUiVariantRef.current = c.uiVariant;
        }
      })
      .catch((err) => {
        setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
      });
    api.hasPin(username).then(setPinSet).catch(() => setPinSet(false));
    if (isTauri) {
      api.hasBiometric(username).then(setBiometricSet).catch(() => setBiometricSet(false));
    }
    refreshSshKeys();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [username]);

  // Re-auth gate for changes to how the vault can be unlocked: resolves true only after a correct
  // master password, so an unattended unlocked session cannot quietly add or drop an unlock method.
  function requireMasterPassword(title: string): Promise<boolean> {
    return new Promise((resolve) => setPwPrompt({ title, resolve }));
  }

  async function handleSetPin(e: FormEvent) {
    e.preventDefault();
    if (!(await requireMasterPassword("Set PIN"))) return;
    setSubmitting(true);
    try {
      setError(null);
      await api.setPin(pinInput);
      setPinInput("");
      setPinSet(true);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleClearPin() {
    if (!(await requireMasterPassword("Remove PIN"))) return;
    setSubmitting(true);
    try {
      setError(null);
      await api.clearPin();
      setPinSet(false);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  // B1-6: the change-master-password command re-verifies currentPwInput itself server-side (same
  // invalid_credentials error every other wrong-password path returns), so there's no separate
  // requireMasterPassword() reprompt here - typing it IS the re-auth. Clears the PIN/biometric
  // inputs client-side too since the backend already dropped both records (they wrapped the old
  // key) - refreshing pinSet/biometricSet from a stale local "true" would show a Remove button
  // for an unlock method that no longer exists.
  // B6: this used the shared page-level `error` state, which renders at the very top of the
  // Settings page (:489-ish) - with this form scrolled into view further down, that message could
  // render off-screen above the visible area (M9.1/M9.2's W finding; same DOM shape on desktop,
  // just not hit whenever the tester's window happened to still show the top). Uses its own local
  // error, rendered directly under the form, so it's never scrolled out of view. On success, force
  // a real logout (lock()) rather than leaving the vault open with the old session - the forced
  // navigation to Login is itself an unmissable result, which also means a same-as-old submission
  // (M9.5) can no longer look like a silent no-op: something visibly happens either way.
  async function handleChangeMasterPassword(e: FormEvent) {
    e.preventDefault();
    if (newPwInput !== confirmPwInput) {
      setPwChangeError("New passwords don't match.");
      return;
    }
    setChangingPassword(true);
    try {
      setPwChangeError(null);
      await api.changeMasterPassword(currentPwInput, newPwInput);
      setCurrentPwInput("");
      setNewPwInput("");
      setConfirmPwInput("");
      setPinSet(false);
      setBiometricSet(false);
      setPasswordChanged(true);
      await api.lock();
    } catch (err) {
      setPwChangeError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setChangingPassword(false);
    }
  }

  async function handleBiometricEnabledChange(enabled: boolean) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateBiometricEnabled(enabled);
      setBiometricEnabled(enabled);
      if (!enabled) setBiometricSet(false);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleSetBiometric() {
    setSubmitting(true);
    try {
      setError(null);
      await api.setBiometric();
      setBiometricSet(true);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleClearBiometric() {
    if (!(await requireMasterPassword("Remove Windows Hello unlock"))) return;
    setSubmitting(true);
    try {
      setError(null);
      await api.clearBiometric();
      setBiometricSet(false);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleSshAgentEnabledChange(enabled: boolean) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateSshAgentEnabled(enabled);
      setSshAgentEnabled(enabled);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleAddSshKey(e: FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    try {
      setError(null);
      await api.addSshKey(sshLabelInput, sshKeyInput);
      setSshLabelInput("");
      setSshKeyInput("");
      refreshSshKeys();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleDeleteSshKey(id: number) {
    if (!(await requireMasterPassword("Delete SSH key"))) return;
    setSubmitting(true);
    try {
      setError(null);
      await api.deleteSshKey(id);
      refreshSshKeys();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

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

  async function handleUiVariantChange(value: string) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateUiVariant(value);
      setUiVariant(value);
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleRepromptModeChange(value: RepromptMode) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateRepromptMode(value);
      setRepromptMode(value);
      // Without this the running session's SecretField instances would keep the stale policy
      // until the next app start, which reads as the setting being broken.
      await refreshRepromptConfig();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleRepromptGraceChange(seconds: number) {
    editedRef.current = true;
    setSubmitting(true);
    try {
      setError(null);
      await api.updateRepromptGraceSeconds(seconds);
      setRepromptGraceSeconds(seconds);
      await refreshRepromptConfig();
    } catch (err) {
      setError(isCommandError(err) ? err.message : "Something went wrong. Please try again.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden bg-mk-base">
      <div className="h-14 px-6 flex items-center gap-4 flex-shrink-0 border-b border-mk-border">
        <div className="w-9 h-9 rounded-xl flex items-center justify-center flex-shrink-0 bg-mk-accent-muted">
          <IconMonitor size={18} className="text-mk-accent" />
        </div>
        <div>
          <h1 className="text-sm font-bold text-mk-fg">Settings</h1>
          <p className="text-xs text-mk-muted">Preferences and security configuration</p>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-2xl mx-auto space-y-3">
          {error && <p className="text-sm text-mk-red">{error}</p>}

          <Section icon={IconLock} title="Security" defaultOpen>
            <Field label="Auto-lock after idle for" desc="How long to wait before auto-locking the vault">
              <select className={SELECT_CLS} value={autoLockMinutes ?? ""} onChange={(e) => handleChange(Number(e.target.value))} disabled={submitting}>
                {OPTIONS.map((m) => (
                  <option key={m} value={m}>
                    {m} minute{m === 1 ? "" : "s"}
                  </option>
                ))}
              </select>
            </Field>
            <Divider />
            <Field label='Consider a password "old" after' desc="Threshold used by the Health Dashboard's Old card">
              <select className={SELECT_CLS} value={oldPasswordDays ?? ""} onChange={(e) => handleOldPasswordDaysChange(Number(e.target.value))} disabled={submitting}>
                {OLD_PASSWORD_DAYS_OPTIONS.map((d) => (
                  <option key={d} value={d}>
                    {d} days
                  </option>
                ))}
              </select>
            </Field>
            <Divider />
            <Field label="Breach check (HIBP)" desc="Opt in to checking passwords against Have I Been Pwned (k-anonymity, no plaintext leaves this app)">
              <Toggle checked={hibpOptIn ?? false} onChange={handleHibpOptInChange} disabled={submitting} />
            </Field>
            <Divider />
            <Field label="PIN unlock" desc={pinSet ? "A PIN is set up for quick unlock." : "Unlock with a short PIN instead of the master password"}>
              {pinSet === null ? null : pinSet ? (
                <button type="button" onClick={handleClearPin} disabled={submitting} title="Remove PIN unlock; only the master password will unlock the vault" className={BTN_SECONDARY_CLS}>
                  Remove PIN
                </button>
              ) : (
                <form onSubmit={handleSetPin} className="flex items-center gap-2">
                  <input
                    type="password"
                    inputMode="numeric"
                    className={`${INPUT_CLS} w-28`}
                    value={pinInput}
                    onChange={(e) => setPinInput(e.target.value)}
                    placeholder="New PIN"
                    title="A PIN for faster local unlock, instead of typing the master password every time"
                  />
                  <button type="submit" disabled={submitting || pinInput.trim() === ""} className={BTN_PRIMARY_CLS}>
                    Set PIN
                  </button>
                </form>
              )}
            </Field>
            {isTauri && (
              <>
                <Divider />
                <Field label="Windows Hello / biometric" desc="Master on/off switch for biometric unlock. New native OS integration - if it misbehaves on your machine, turn this off.">
                  <Toggle checked={biometricEnabled ?? false} onChange={handleBiometricEnabledChange} disabled={submitting} />
                </Field>
                {biometricEnabled && (
                  <Field label="Biometric setup" desc={biometricSet ? "Biometric unlock is set up." : "Prompts Windows Hello to confirm, then enables biometric unlock"}>
                    {biometricSet === null ? null : biometricSet ? (
                      <button type="button" onClick={handleClearBiometric} disabled={submitting} title="Remove biometric unlock" className={BTN_SECONDARY_CLS}>
                        Remove
                      </button>
                    ) : (
                      <button type="button" onClick={handleSetBiometric} disabled={submitting} className={BTN_PRIMARY_CLS}>
                        Set up
                      </button>
                    )}
                  </Field>
                )}
                <Field label="Remember last username" desc="Shows the Windows Hello / PIN option for that username immediately on the Login screen, without waiting for it to be typed. Stored on this device only, not synced or encrypted - a username isn't a secret.">
                  <Toggle
                    checked={rememberLastUsername}
                    onChange={(v) => {
                      setRememberLastUsername(v);
                      setRememberLastUsernameStored(v);
                    }}
                  />
                </Field>
              </>
            )}
            <Divider />
            <div className="pt-1">
              <p className="text-sm font-medium text-mk-fg">Change master password</p>
              <p className="text-xs mt-0.5 text-mk-muted">
                Re-seals every stored secret under the new password (a backup is taken automatically first). Removes PIN and biometric unlock -
                set them up again afterward.
              </p>
              {/* N1: same no-recovery caution as Register - forgetting a NEW master password is
                  just as unrecoverable as forgetting the original one. */}
              <p className="text-xs mt-1 text-yellow-500">
                There is no password recovery. Choose a new master password you won't forget - if
                you do, this vault cannot be decrypted by anyone, including you.
              </p>
              <form onSubmit={handleChangeMasterPassword} className="mt-3 space-y-2 max-w-xs">
                <input
                  type="password"
                  placeholder="Current master password"
                  autoComplete="current-password"
                  className={INPUT_CLS}
                  value={currentPwInput}
                  onChange={(e) => setCurrentPwInput(e.target.value)}
                  disabled={changingPassword}
                />
                <input
                  type="password"
                  placeholder="New master password"
                  autoComplete="new-password"
                  className={INPUT_CLS}
                  value={newPwInput}
                  onChange={(e) => setNewPwInput(e.target.value)}
                  disabled={changingPassword}
                />
                <input
                  type="password"
                  placeholder="Confirm new master password"
                  autoComplete="new-password"
                  className={INPUT_CLS}
                  value={confirmPwInput}
                  onChange={(e) => setConfirmPwInput(e.target.value)}
                  disabled={changingPassword}
                />
                <button
                  type="submit"
                  disabled={changingPassword || !currentPwInput || !newPwInput || !confirmPwInput}
                  title="Re-encrypts the whole vault under the new password - takes a moment on a large vault"
                  className={BTN_PRIMARY_CLS}
                >
                  {changingPassword ? "Changing…" : "Change Password"}
                </button>
                {passwordChanged && <p className="text-sm text-mk-green">Master password changed.</p>}
                {pwChangeError && <p className="text-sm text-mk-red">{pwChangeError}</p>}
              </form>
            </div>
          </Section>

          {isTauri && (
            <Section icon={IconTerminal} title="SSH Agent">
              <Field label="Enable SSH agent" desc="Master on/off switch for the SSH agent (Windows named pipe). New native OS integration, untested against a real SSH client - turn off if it misbehaves.">
                <Toggle checked={sshAgentEnabled ?? false} onChange={handleSshAgentEnabledChange} disabled={submitting} />
              </Field>
              {sshAgentEnabled && (
                <>
                  <Divider />
                  <div>
                    <p className="text-xs font-semibold uppercase tracking-widest mb-3 text-mk-faint">Stored Keys</p>
                    <div className="space-y-2">
                      {(sshKeys ?? []).map((k) => (
                        <div key={k.id} className="flex items-center gap-3 p-3 rounded-xl border border-mk-border bg-mk-panel">
                          <div className="flex-1 min-w-0">
                            <p className="text-sm font-medium text-mk-fg2 truncate">{k.label}</p>
                            <p className="text-xs truncate font-mk-mono text-mk-faint" title={k.publicKey}>
                              {k.publicKey}
                            </p>
                          </div>
                          <button type="button" title="Delete this key" onClick={() => handleDeleteSshKey(k.id)} className="w-7 h-7 flex items-center justify-center rounded-lg text-mk-red hover:bg-mk-badge flex-shrink-0">
                            <IconTrash size={13} />
                          </button>
                        </div>
                      ))}
                      {sshKeys?.length === 0 && <p className="text-sm text-mk-faint">No SSH keys added.</p>}
                    </div>

                    <form onSubmit={handleAddSshKey} className="mt-3 space-y-2">
                      <input className={INPUT_CLS} placeholder="Label" value={sshLabelInput} onChange={(e) => setSshLabelInput(e.target.value)} />
                      <textarea
                        className={`${INPUT_CLS} font-mk-mono`}
                        rows={4}
                        placeholder={"-----BEGIN OPENSSH PRIVATE KEY-----\n...\n-----END OPENSSH PRIVATE KEY-----"}
                        value={sshKeyInput}
                        onChange={(e) => setSshKeyInput(e.target.value)}
                        title="Unencrypted OpenSSH private key (no passphrase)"
                      />
                      <button type="submit" disabled={submitting || sshLabelInput.trim() === "" || sshKeyInput.trim() === ""} className={BTN_PRIMARY_CLS}>
                        Add key
                      </button>
                    </form>
                  </div>
                </>
              )}
            </Section>
          )}

          <Section icon={IconPalette} title="Appearance" defaultOpen>
            <Field label="Theme" desc="Light or dark appearance">
              <select className={SELECT_CLS} value={theme ?? ""} onChange={(e) => handleThemeChange(e.target.value)} disabled={submitting}>
                <option value="dark">Dark</option>
                <option value="light">Light</option>
              </select>
            </Field>
            <Divider />
            <Field label="Accent color" desc="Highlight color used throughout the app">
              <select className={SELECT_CLS} value={accentColor ?? ""} onChange={(e) => handleAccentChange(e.target.value)} disabled={submitting}>
                {ACCENT_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </Field>
            <Divider />
            <Field label="Mobile navigation style" desc="How the sidebar is replaced on narrow screens/windows below 640px">
              <select className={SELECT_CLS} value={mobileNavStyle ?? ""} onChange={(e) => handleMobileNavStyleChange(e.target.value)} disabled={submitting}>
                {MOBILE_NAV_STYLE_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </Field>
          </Section>

          <Section icon={IconMonitor} title="Interface" defaultOpen>
            <Field label="Interface" desc="Which vault interface this app opens - takes effect on next launch">
              <select className={SELECT_CLS} value={uiVariant ?? ""} onChange={(e) => handleUiVariantChange(e.target.value)} disabled={submitting}>
                {UI_VARIANT_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </Field>
            {bootUiVariantRef.current !== null && bootUiVariantRef.current !== uiVariant && (
              <p className="text-sm rounded-lg border border-mk-border px-3 py-2 text-mk-fg2">
                Interface changed to {UI_VARIANT_OPTIONS.find((o) => o.value === uiVariant)?.label}.{" "}
                {isTauri ? "Restart the app to see it." : `Open /${uiVariant === "classic" ? "index" : `${uiVariant}ui`}.html in this browser to use it.`}
              </p>
            )}
          </Section>

          <Section icon={IconDatabase} title="Vault" defaultOpen>
            <Field label="Password strength algorithm" desc="Which algorithm scores password strength">
              <select className={SELECT_CLS} value={strengthAlgorithm ?? ""} onChange={(e) => handleAlgorithmChange(e.target.value)} disabled={submitting}>
                {ALGORITHM_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </Field>
            <Divider />
            <Field
              label="Reprompt mode"
              desc="Every item's reveal/copy asks for the master password by default (a per-item checkbox can exempt one item). Grace window: one correct entry suppresses further prompts for a while. Always: no grace window, every reveal prompts."
            >
              <select className={SELECT_CLS} value={repromptMode ?? ""} onChange={(e) => handleRepromptModeChange(e.target.value as RepromptMode)} disabled={submitting}>
                {REPROMPT_MODE_OPTIONS.map((o) => (
                  <option key={o.value} value={o.value}>
                    {o.label}
                  </option>
                ))}
              </select>
            </Field>
            <Divider />
            <Field label="Reprompt grace window" desc="Applies to Grace-window mode only: how long after one correct master password entry before the next reveal/copy asks again.">
              <select
                className={SELECT_CLS}
                value={repromptGraceSeconds ?? ""}
                onChange={(e) => handleRepromptGraceChange(Number(e.target.value))}
                disabled={submitting || repromptMode !== "grace"}
              >
                {REPROMPT_GRACE_OPTIONS.map((s) => (
                  <option key={s} value={s}>
                    {s === 300 ? "300 seconds (5 minutes)" : `${s} seconds`}
                  </option>
                ))}
              </select>
            </Field>
          </Section>
        </div>
      </div>

      {pwPrompt && (
        <MasterPasswordPrompt
          title={pwPrompt.title}
          onResult={(ok) => {
            pwPrompt.resolve(ok);
            setPwPrompt(null);
          }}
        />
      )}
    </div>
  );
}
