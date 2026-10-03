# Make UI: unify all copy buttons to the "Copy username" look

## Context
In Make UI, `CopyButton` (username / URL / plain fields) is icon-only: 28x28, `rounded-md text-mk-muted bg-mk-badge hover:opacity-80`, `IconCopy size=12`. Other copy buttons still use old bordered style (`border rounded px-2 py-0.5`, clipboard icon, some with "Copy" text). Goal: every copy button in Make UI looks identical to the username one. Look only. Behavior (reprompt gate, 30s auto-clear, toasts) unchanged.

## Copy buttons to restyle (Make UI only)
| Where (Make UI) | Source | Now |
|---|---|---|
| Password in credential block, password history rows, hidden custom fields | `src/components/SecretField.tsx:130` | bordered, clipboard icon + "Copy" text |
| TOTP code | `src/components/TotpDisplay.tsx:35` | bordered, clipboard icon |
| Generator page + item drawer generator | `src/components/PasswordGeneratorPanel.tsx:208` | bordered, icon + "Copy" |

Already correct (untouched): `src/makeui/components/CopyButton.tsx` (username, URL, DetailField non-hidden).

## Approach
Those 3 components are shared with old UI + newui, so old UIs must not change.
1. Add optional prop `variant?: "make"` to `SecretField`, `TotpDisplay`, `PasswordGeneratorPanel`. When `"make"`, the copy button renders with the exact `CopyButton` classes + `<IconCopy size={12} />` (from `src/makeui/icons.tsx`), icon-only, keeping existing `title` and `onClick`.
2. Pass `variant="make"` at Make UI call sites: `makeui/components/CredentialBlock.tsx` (lines ~196, 202, 246), `makeui/components/DetailField.tsx:26`, `makeui/components/Generator.tsx:49`, `makeui/components/ItemDrawer.tsx:263`.
3. Eye (show/hide) button: left as is (not a copy button). Say so if wanted restyled too.
4. To avoid duplicated class string drift, export the class constant from `makeui/components/CopyButton.tsx` and reuse it.

## Verify
- `npx tsc --noEmit` and `npm test` (parity/vitest suites) pass.
- Run app in Make UI (`makeui.html`), open credential: username, password, history, TOTP, custom hidden field, generator page, item drawer all show identical 28px icon-only button; copy still triggers reprompt + toast.
- Old UI / newui screens visually unchanged.

## Note
On execution, copy this plan to `docs/Plans/` (user pref: no C-drive plans).
