# How to Test — SSH Agent

Manual QA checklist for the SSH agent, added on `feature/bitwarden-port-phase1`. The signing logic
itself (`core/src/ssh_agent.rs`) is unit-tested with a real generated Ed25519 key, including a full
sign+verify round-trip — that part is trustworthy. What's **not** verified is the actual named-pipe
server (`ssh-agent-lib` crate) and real interop with an OpenSSH or PuTTY client, since this project
has no way to drive a real SSH client against a live named pipe in an automated build.

Defaults **off** (`ssh_agent_enabled: false` in config). Nothing here runs unless you explicitly
turn it on in Settings. Desktop app only — no web-mode route.

## Prerequisites

- Windows with an OpenSSH client (`ssh`, `ssh-keygen`, `ssh-add -l` all ship with modern Windows —
  `Get-Command ssh` to confirm) or PuTTY/Pageant if you prefer that stack.
- A throwaway SSH key pair to test with — don't use a real production key for this first pass:
  ```powershell
  ssh-keygen -t ed25519 -f $env:TEMP\pwmgr-test-key -N '""'
  ```
  This generates `pwmgr-test-key` (private, **no passphrase** — the agent doesn't support encrypted
  keys yet, see below) and `pwmgr-test-key.pub`.
- A target to test signing against. Easiest: add the test key's public key to your own
  `~/.ssh/authorized_keys` and SSH to `localhost` (if an SSH server is running), or just use
  `ssh-add -l` / `ssh-keygen -y -f` style local checks that don't need a live server if you don't
  have one set up.

```powershell
npm run tauri dev
```

## Checklist

- [ ] **Feature is off by default.** Settings → Security → SSH Agent section: "Enable SSH agent"
      unchecked, no key list or add-key form visible.
- [ ] **Enabling starts the listener live, no restart needed.** Check "Enable SSH agent". Then, in
      a separate terminal:
      ```powershell
      Get-ChildItem \\.\pipe\ | Select-String pwmgr-ssh-agent
      ```
      (or `[System.IO.Directory]::GetFiles("\\.\pipe\")` in PowerShell if `Get-ChildItem` can't
      list named pipes on your system) — the pipe should exist immediately after checking the box,
      without restarting the app.
- [ ] **Adding a key validates it.**
  - [ ] Paste the *public* key or garbage text into the private-key field → clear error, not a
        crash, key not added.
  - [ ] Paste an actual **passphrase-encrypted** private key → clear error mentioning it needs to
        be decrypted first (this is a known, deliberate v1 limitation — confirm the error message
        is accurate, not a generic failure).
  - [ ] Paste the real unencrypted test private key + a label → succeeds, appears in the list
        below with the label and a public-key line starting `ssh-ed25519 AAAA...`.
  - [ ] Confirm the public-key line shown in Settings matches `Get-Content $env:TEMP\pwmgr-test-key.pub`
        exactly.
- [ ] **List identities via a real client.** With the vault **unlocked** and the key added, run:
      ```powershell
      $env:SSH_AUTH_SOCK = "\\.\pipe\pwmgr-ssh-agent"
      ssh-add -l
      ```
      Your test key's fingerprint/comment (the label you set) should show up. If `ssh-add` on your
      Windows build doesn't support named-pipe `SSH_AUTH_SOCK` directly, note that as a finding —
      it may need a bridge (e.g. via WSL or a helper) rather than being a bug in this app.
- [ ] **Locked vault yields no identities, not an error.** Lock the vault, run `ssh-add -l` again
      (or re-list identities) — should show "no identities" (or empty), not crash or hang the
      calling client.
- [ ] **Signing prompts for approval.** With the vault unlocked, attempt an actual SSH operation
      that needs this key (e.g. `ssh -o IdentitiesOnly=yes -i pwmgr-test-key-fingerprint localhost`
      if you set up `authorized_keys`, or any tool that calls out to the agent to sign). A native
      dialog ("An application is requesting to sign with SSH key '<label>'. Allow?") should appear
      *before* the sign completes.
  - [ ] Click **Yes** → signing succeeds, the SSH operation proceeds normally.
  - [ ] Click **No** (or close the dialog) → signing is refused, the SSH client reports an auth
        failure, the app doesn't crash.
- [ ] **Deleting a key removes it live.** Delete the key from Settings, re-run `ssh-add -l` —
      it should no longer appear, without restarting the app.
- [ ] **Disabling the feature stops the listener.** Uncheck "Enable SSH agent". Re-check for the
      named pipe (first bullet's command) — it should be gone. `ssh-add -l` against the stale
      socket path should now fail to connect rather than hang.
- [ ] **Re-enabling after disabling works.** Toggle it back on, confirm the pipe comes back and a
      previously-added key is still listed (deletion is the only thing that should remove a key;
      toggling the feature off/on must not lose stored keys).

## Known, deliberate v1 gaps (not bugs — don't file these, just be aware)

- No in-app key generation — import an existing key only.
- No passphrase-encrypted key support.
- No trash/undo for deleted SSH keys (unlike credentials) — deletion is immediate and permanent.

If the named pipe or signing flow misbehaves: the "Enable SSH agent" checkbox is the escape hatch.
Turning it off should always leave the rest of the app (vault, credentials, everything else on
this branch) completely unaffected.
