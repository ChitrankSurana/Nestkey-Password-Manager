import { useState, createContext, useContext, useCallback, useEffect, useRef } from "react";

// ── Theme context ──────────────────────────────────────────────────────────
const ThemeCtx = createContext<{ dark: boolean; toggle: () => void }>({ dark: true, toggle: () => {} });
function useTheme() { return useContext(ThemeCtx); }

type TMap = {
  bgBase: string; bgSidebar: string; bgPanel: string; bgCard: string; bgInput: string;
  border: string; borderHover: string;
  textPrimary: string; textSecondary: string; textMuted: string; textFaint: string;
  accent: string; accentLight: string; accentMuted: string;
  itemHover: string; itemSelected: string; itemSelectedBorder: string;
  sectionLabel: string; badge: string; badgeActive: string;
};
const DARK: TMap = {
  bgBase: "#13151c", bgSidebar: "#1a1d27", bgPanel: "#1e2130", bgCard: "#252839", bgInput: "#13151c",
  border: "rgba(255,255,255,0.06)", borderHover: "rgba(255,255,255,0.14)",
  textPrimary: "#dde1ef", textSecondary: "#c0c6e0", textMuted: "#7c82a0", textFaint: "#3a3f58",
  accent: "#175ddc", accentLight: "#5b9cf6", accentMuted: "rgba(23,93,220,0.18)",
  itemHover: "rgba(255,255,255,0.03)", itemSelected: "rgba(23,93,220,0.18)", itemSelectedBorder: "#175ddc",
  sectionLabel: "#3a3f58", badge: "rgba(255,255,255,0.06)", badgeActive: "rgba(23,93,220,0.3)",
};
const LIGHT: TMap = {
  bgBase: "#f4f5f9", bgSidebar: "#ffffff", bgPanel: "#f0f1f6", bgCard: "#ffffff", bgInput: "#f4f5f9",
  border: "rgba(0,0,0,0.07)", borderHover: "rgba(0,0,0,0.14)",
  textPrimary: "#1a1c2a", textSecondary: "#383c54", textMuted: "#6b6f88", textFaint: "#b0b4cc",
  accent: "#175ddc", accentLight: "#1252b8", accentMuted: "rgba(23,93,220,0.1)",
  itemHover: "rgba(0,0,0,0.03)", itemSelected: "rgba(23,93,220,0.08)", itemSelectedBorder: "#175ddc",
  sectionLabel: "#b0b4cc", badge: "rgba(0,0,0,0.06)", badgeActive: "rgba(23,93,220,0.15)",
};

// ── Toast context ──────────────────────────────────────────────────────────
interface ToastItem { id: string; message: string; type: "success" | "error" | "info"; }
const ToastCtx = createContext<(msg: string, type?: ToastItem["type"]) => void>(() => {});
function useToast() { return useContext(ToastCtx); }

function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const push = useCallback((message: string, type: ToastItem["type"] = "success") => {
    const id = Math.random().toString(36).slice(2);
    setToasts(t => [...t, { id, message, type }]);
    setTimeout(() => setToasts(t => t.filter(x => x.id !== id)), 3000);
  }, []);
  return (
    <ToastCtx.Provider value={push}>
      {children}
      <div className="fixed bottom-4 right-4 flex flex-col gap-2 z-50 pointer-events-none">
        {toasts.map(t => (
          <div
            key={t.id}
            className="px-4 py-2.5 rounded-xl text-xs font-medium shadow-lg pointer-events-auto transition-all"
            style={{
              background: t.type === "success" ? "rgba(45,204,143,0.15)" : t.type === "error" ? "rgba(248,78,78,0.15)" : "rgba(23,93,220,0.15)",
              color: t.type === "success" ? "#2dcc8f" : t.type === "error" ? "#f84e4e" : "#5b9cf6",
              border: `1px solid ${t.type === "success" ? "rgba(45,204,143,0.3)" : t.type === "error" ? "rgba(248,78,78,0.3)" : "rgba(23,93,220,0.3)"}`,
              backdropFilter: "blur(8px)",
            }}
          >
            {t.message}
          </div>
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

// ── Confirm dialog ─────────────────────────────────────────────────────────
interface ConfirmDialogProps {
  open: boolean;
  title: string;
  body: string;
  confirmLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
}
function ConfirmDialog({ open, title, body, confirmLabel = "Confirm", onConfirm, onCancel }: ConfirmDialogProps) {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ background: "rgba(0,0,0,0.6)", backdropFilter: "blur(4px)" }}>
      <div className="w-full max-w-sm mx-4 p-6 rounded-2xl shadow-2xl" style={{ background: T.bgCard, border: `1px solid ${T.border}` }}>
        <h3 className="text-sm font-semibold mb-2" style={{ color: T.textPrimary }}>{title}</h3>
        <p className="text-xs leading-relaxed mb-6" style={{ color: T.textMuted }}>{body}</p>
        <div className="flex gap-3 justify-end">
          <button
            onClick={onCancel}
            className="px-4 py-2 rounded-lg text-xs font-medium transition-all"
            style={{ background: T.badge, color: T.textSecondary }}
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            className="px-4 py-2 rounded-lg text-xs font-semibold text-white transition-all bg-[#f84e4e] hover:bg-[#d93e3e]"
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Types ──────────────────────────────────────────────────────────────────
type ItemType = "login" | "note" | "card" | "identity";
type NavSection = "all" | "favorites" | "logins" | "notes" | "cards" | "identities" | "health" | "generator" | "data" | "trash" | "settings" | string;

interface CustomField { label: string; value: string; fieldType: "text" | "hidden" | "boolean"; }
interface Credential { id: string; username?: string; password?: string; totp?: string; reprompt?: boolean; customFields?: CustomField[]; }
interface VaultItem {
  id: string; type: ItemType; name: string;
  username?: string; password?: string; website?: string;
  cardNumber?: string; cardExpiry?: string; cardCVV?: string;
  noteContent?: string; folder?: string; favorite: boolean;
  totp?: string; reprompt?: boolean; customFields?: CustomField[];
  credentials?: Credential[];
  lastModified: string; createdAt: string; trashed?: boolean;
}

// ── Sample data ────────────────────────────────────────────────────────────
const VAULT_ITEMS: VaultItem[] = [
  {
    id: "1", type: "login", name: "GitHub", username: "maya.chen@dev.io",
    password: "G!th3b#S3cur3P@ss2024", website: "https://github.com", folder: "Development",
    favorite: true, totp: "JBSWY3DPEHPK3PXP", lastModified: "2024-12-18", createdAt: "2023-03-12",
    reprompt: true,
    customFields: [
      { label: "Recovery Code 1", value: "8842-1923-0012", fieldType: "hidden" },
      { label: "Recovery Code 2", value: "9912-4412-8812", fieldType: "hidden" }
    ]
  },
  {
    id: "2", type: "login", name: "AWS Console",
    website: "https://console.aws.amazon.com", folder: "Development",
    favorite: true, lastModified: "2025-01-02", createdAt: "2022-08-05",
    credentials: [
      { id: "c1", username: "m.chen+aws@company.io", password: "4ws!C0ns0l3#Admin99", reprompt: true },
      { id: "c2", username: "root@company.io", password: "R00t#Br34kGl4ss!!", totp: "OIRUQW238478SDF" }
    ]
  },
  {
    id: "3", type: "login", name: "Figma", username: "maya.chen@dev.io",
    password: "F!gm@Des!gn2024#", website: "https://figma.com", folder: "Design",
    favorite: false, lastModified: "2024-11-30", createdAt: "2023-06-20",
  },
  {
    id: "4", type: "login", name: "Linear", username: "maya.chen@dev.io",
    password: "L!n34r#Track3r!", website: "https://linear.app", folder: "Development",
    favorite: false, lastModified: "2024-12-05", createdAt: "2023-09-01",
  },
  {
    id: "5", type: "login", name: "1Password (backup)", username: "maya.chen@personal.com",
    password: "B4ckup#P4ss!2024", website: "https://1password.com", folder: "Personal",
    favorite: false, lastModified: "2024-10-14", createdAt: "2021-11-30",
  },
  {
    id: "6", type: "login", name: "Vercel", username: "maya.chen@dev.io",
    password: "V3rc3l!D3pl0y#99", website: "https://vercel.com", folder: "Development",
    favorite: true, lastModified: "2025-01-08", createdAt: "2023-04-15",
  },
  {
    id: "7", type: "login", name: "Notion", username: "maya.chen@dev.io",
    password: "N0t!0n#W0rks2024", website: "https://notion.so", folder: "Work",
    favorite: false, lastModified: "2024-12-22", createdAt: "2022-02-11",
  },
  {
    id: "8", type: "note", name: "SSH Key Passphrase",
    noteContent: "Primary key passphrase: refer to physical notebook. Backup key stored in encrypted USB drive in desk drawer.",
    folder: "Development", favorite: false, lastModified: "2024-09-10", createdAt: "2022-05-03",
  },
  {
    id: "9", type: "note", name: "Bank Account Numbers",
    noteContent: "Checking: ****4821 | Savings: ****0093 | Routing: 0210****. Full numbers in encrypted partition.",
    folder: "Finance", favorite: false, lastModified: "2024-11-20", createdAt: "2021-07-14",
  },
  {
    id: "10", type: "card", name: "Chase Sapphire Reserve",
    cardNumber: "4*** **** **** 3921", cardExpiry: "09/27", cardCVV: "***",
    folder: "Finance", favorite: true, lastModified: "2024-08-30", createdAt: "2020-09-01",
  },
  {
    id: "11", type: "card", name: "Amex Gold",
    cardNumber: "3*** ****** *8204", cardExpiry: "03/26", cardCVV: "****",
    folder: "Finance", favorite: false, lastModified: "2024-03-12", createdAt: "2021-03-05",
  },
  {
    id: "12", type: "identity", name: "Maya Chen — Passport",
    username: "Maya Lan Chen", folder: "Documents",
    favorite: false, lastModified: "2024-05-18", createdAt: "2022-12-01",
  },
  {
    id: "13", type: "login", name: "Cloudflare", username: "maya.chen@dev.io",
    password: "Cl0udf!@re#DNS2024", website: "https://dash.cloudflare.com", folder: "Development",
    favorite: false, lastModified: "2024-12-01", createdAt: "2023-02-28", trashed: true
  },
  {
    id: "14", type: "login", name: "Stripe Dashboard", username: "maya.chen@company.io",
    password: "Str!pe#Pay2024!!", website: "https://dashboard.stripe.com", folder: "Work",
    favorite: false, lastModified: "2024-11-15", createdAt: "2023-07-09", trashed: true
  },
];

const FOLDERS = ["Development", "Design", "Finance", "Personal", "Work", "Documents"];

// ── Icons ──────────────────────────────────────────────────────────────────
type IconProps = { size?: number; className?: string; style?: React.CSSProperties };
function IconKey({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><circle cx="7.5" cy="15.5" r="5.5" /><path d="m21 2-9.6 9.6" /><path d="m15.5 7.5 3 3L22 7l-3-3" /></svg>;
}
function IconNote({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline points="14 2 14 8 20 8" /><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" /></svg>;
}
function IconCard({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><rect x="1" y="4" width="22" height="16" rx="2" ry="2" /><line x1="1" y1="10" x2="23" y2="10" /></svg>;
}
function IconPerson({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" /><circle cx="12" cy="7" r="4" /></svg>;
}
function IconStar({ size = 16, filled = false, className = "", style }: IconProps & { filled?: boolean }) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill={filled ? "currentColor" : "none"} stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" /></svg>;
}
function IconFolder({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" /></svg>;
}
function IconSearch({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" /></svg>;
}
function IconCopy({ size = 14, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg>;
}
function IconEye({ size = 14, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg>;
}
function IconEyeOff({ size = 14, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" /><path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" /><line x1="1" y1="1" x2="23" y2="23" /></svg>;
}
function IconLock({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><rect x="3" y="11" width="18" height="11" rx="2" ry="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" /></svg>;
}
function IconSettings({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>;
}
function IconActivity({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><polyline points="22 12 18 12 15 21 9 3 6 12 2 12" /></svg>;
}
function IconZap({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" /></svg>;
}
function IconDatabase({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><ellipse cx="12" cy="5" rx="9" ry="3" /><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" /><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" /></svg>;
}
function IconPlus({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" /></svg>;
}
function IconTrash({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><polyline points="3 6 5 6 21 6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2" /></svg>;
}
function IconChevron({ size = 14, down = false, right = false, className = "", style }: IconProps & { down?: boolean; right?: boolean }) {
  const rot = down ? "rotate(90deg)" : right ? "rotate(0deg)" : "";
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={{ transform: rot, ...style }}><polyline points="9 18 15 12 9 6" /></svg>;
}
function IconShield({ size = 20, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" /></svg>;
}
function IconCheck({ size = 14, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><polyline points="20 6 9 17 4 12" /></svg>;
}
function IconSun({ size = 15, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><circle cx="12" cy="12" r="5" /><line x1="12" y1="1" x2="12" y2="3" /><line x1="12" y1="21" x2="12" y2="23" /><line x1="4.22" y1="4.22" x2="5.64" y2="5.64" /><line x1="18.36" y1="18.36" x2="19.78" y2="19.78" /><line x1="1" y1="12" x2="3" y2="12" /><line x1="21" y1="12" x2="23" y2="12" /><line x1="4.22" y1="19.78" x2="5.64" y2="18.36" /><line x1="18.36" y1="5.64" x2="19.78" y2="4.22" /></svg>;
}
function IconMoon({ size = 15, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" /></svg>;
}
function IconRefresh({ size = 15, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><polyline points="23 4 23 10 17 10" /><polyline points="1 20 1 14 7 14" /><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" /></svg>;
}
function IconUpload({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><polyline points="16 16 12 12 8 16" /><line x1="12" y1="12" x2="12" y2="21" /><path d="M20.39 18.39A5 5 0 0 0 18 9h-1.26A8 8 0 1 0 3 16.3" /></svg>;
}
function IconDownload({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><polyline points="8 17 12 21 16 17" /><line x1="12" y1="12" x2="12" y2="21" /><path d="M20.88 18.09A5 5 0 0 0 18 9h-1.26A8 8 0 1 0 3 16.29" /></svg>;
}
function IconWifi({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><line x1="1" y1="1" x2="23" y2="23" /><path d="M16.72 11.06A10.94 10.94 0 0 1 19 12.55" /><path d="M5 12.55a10.94 10.94 0 0 1 5.17-2.39" /><path d="M10.71 5.05A16 16 0 0 1 22.56 9" /><path d="M1.42 9a15.91 15.91 0 0 1 4.7-2.88" /><path d="M8.53 16.11a6 6 0 0 1 6.95 0" /><line x1="12" y1="20" x2="12.01" y2="20" /></svg>;
}
function IconKey2({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" /></svg>;
}
function IconAlertTriangle({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" /><line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" /></svg>;
}
function IconTerminal({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" /></svg>;
}
function IconPalette({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><circle cx="13.5" cy="6.5" r=".5" /><circle cx="17.5" cy="10.5" r=".5" /><circle cx="8.5" cy="7.5" r=".5" /><circle cx="6.5" cy="12.5" r=".5" /><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z" /></svg>;
}
function IconMonitor({ size = 16, className = "", style }: IconProps) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={className} style={style}><rect x="2" y="3" width="20" height="14" rx="2" ry="2" /><line x1="8" y1="21" x2="16" y2="21" /><line x1="12" y1="17" x2="12" y2="21" /></svg>;
}

// ── Item type config ───────────────────────────────────────────────────────
function itemTypeConfig(type: ItemType) {
  const map = {
    login: { icon: IconKey, bg: "bg-[#1e3a6e]", color: "text-[#5b9cf6]", label: "Login" },
    note: { icon: IconNote, bg: "bg-[#2d3a1e]", color: "text-[#7ec87e]", label: "Secure Note" },
    card: { icon: IconCard, bg: "bg-[#3a2d1e]", color: "text-[#f0a056]", label: "Payment Card" },
    identity: { icon: IconPerson, bg: "bg-[#2d1e3a]", color: "text-[#b07ef0]", label: "Identity" },
  };
  return map[type];
}

// ── Copy button ────────────────────────────────────────────────────────────
function CopyButton({ value, label }: { value: string; label?: string }) {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  const toast = useToast();
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(value).catch(() => {});
    setCopied(true);
    toast("Copied — clears in 30s");
    setTimeout(() => setCopied(false), 1800);
  };

  if (label) {
    return (
      <button
        onClick={handleCopy}
        style={copied ? { background: "rgba(45,204,143,0.15)", color: "#2dcc8f", borderColor: "rgba(45,204,143,0.3)" } : { background: T.badge, color: T.textMuted, borderColor: T.border }}
        className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium border transition-all"
      >
        {copied ? <IconCheck size={12} /> : <IconCopy size={12} />}
        {copied ? "Copied" : label}
      </button>
    );
  }

  return (
    <button
      onClick={handleCopy}
      style={copied ? { background: "rgba(45,204,143,0.15)", color: "#2dcc8f" } : { background: T.badge, color: T.textMuted }}
      className="flex items-center justify-center w-7 h-7 rounded-md transition-all duration-150"
      title="Copy to clipboard"
    >
      {copied ? <IconCheck size={12} /> : <IconCopy size={12} />}
    </button>
  );
}

// ── Detail field ───────────────────────────────────────────────────────────
const SECRET_MASK = "••••••••••••••••";

function DetailField({ label, value, mono = false, secret = false, reprompted = false }: { label: string; value: string; mono?: boolean; secret?: boolean; reprompted?: boolean }) {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  const [revealed, setRevealed] = useState(false);
  const [repromptOpen, setRepromptOpen] = useState(false);
  const [repromptPass, setRepromptPass] = useState("");

  const display = secret && !revealed ? SECRET_MASK : value;

  const handleReveal = () => {
    if (reprompted && !revealed) {
      setRepromptOpen(true);
    } else {
      setRevealed(r => !r);
    }
  };

  return (
    <>
      {repromptOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center" style={{ background: "rgba(0,0,0,0.6)", backdropFilter: "blur(4px)" }}>
          <div className="w-full max-w-sm mx-4 p-6 rounded-2xl shadow-2xl" style={{ background: dark ? DARK.bgCard : LIGHT.bgCard, border: `1px solid ${T.border}` }}>
            <div className="flex items-center gap-3 mb-4">
              <div className="w-9 h-9 rounded-xl flex items-center justify-center bg-[#175ddc]/10 text-[#175ddc] flex-shrink-0">
                <IconLock size={16} />
              </div>
              <div>
                <h3 className="text-sm font-semibold" style={{ color: T.textPrimary }}>Master Password Required</h3>
                <p className="text-xs" style={{ color: T.textMuted }}>This field requires re-authentication</p>
              </div>
            </div>
            <input
              type="password"
              placeholder="Enter master password"
              value={repromptPass}
              onChange={e => setRepromptPass(e.target.value)}
              onKeyDown={e => { if (e.key === "Enter" && repromptPass) { setRevealed(true); setRepromptOpen(false); setRepromptPass(""); } }}
              className="w-full px-3 py-2.5 text-sm border rounded-xl mb-4 focus:outline-none focus:ring-2 focus:ring-[#175ddc] transition-all"
              style={{ background: T.bgInput, borderColor: T.border, color: T.textPrimary }}
              autoFocus
            />
            <div className="flex gap-3 justify-end">
              <button onClick={() => { setRepromptOpen(false); setRepromptPass(""); }} className="px-4 py-2 rounded-lg text-xs font-medium" style={{ background: T.badge, color: T.textSecondary }}>Cancel</button>
              <button
                onClick={() => { if (repromptPass) { setRevealed(true); setRepromptOpen(false); setRepromptPass(""); } }}
                className="px-4 py-2 rounded-lg text-xs font-semibold text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors"
              >
                Authenticate
              </button>
            </div>
          </div>
        </div>
      )}
      <div className="group">
        <p className="text-[10px] font-medium uppercase tracking-widest mb-1" style={{ color: T.textFaint }}>{label}</p>
        <div className="flex items-center gap-2">
          <p
            className="flex-1 text-sm break-all leading-relaxed select-all"
            style={{ color: T.textSecondary, fontFamily: mono ? "'JetBrains Mono', monospace" : undefined }}
          >
            {display}
          </p>
          <div className="flex items-center gap-1 flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
            {secret && (
              <button
                onClick={handleReveal}
                style={{ background: T.badge, color: T.textMuted }}
                className="flex items-center justify-center w-7 h-7 rounded-md transition-all"
                title={revealed ? "Hide" : "Reveal"}
              >
                {revealed ? <IconEyeOff size={12} /> : <IconEye size={12} />}
              </button>
            )}
            <CopyButton value={value} />
          </div>
        </div>
      </div>
    </>
  );
}

// ── Sidebar ────────────────────────────────────────────────────────────────
function Sidebar({
  activeSection, setActiveSection, foldersOpen, setFoldersOpen, onLock,
}: {
  activeSection: NavSection; setActiveSection: (s: NavSection) => void;
  foldersOpen: boolean; setFoldersOpen: (v: boolean) => void; onLock: () => void;
}) {
  const { dark, toggle } = useTheme();
  const T = dark ? DARK : LIGHT;

  const navItem = (id: NavSection, label: string, Icon: React.FC<{ size?: number; className?: string }>, count?: number) => {
    const active = activeSection === id;
    return (
      <button
        key={id}
        onClick={() => setActiveSection(id)}
        style={{ background: active ? T.accentMuted : "transparent", color: active ? T.accentLight : T.textMuted }}
        className="w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-all duration-150"
        onMouseEnter={e => { if (!active) { (e.currentTarget as HTMLElement).style.background = T.itemHover; (e.currentTarget as HTMLElement).style.color = T.textSecondary; } }}
        onMouseLeave={e => { if (!active) { (e.currentTarget as HTMLElement).style.background = "transparent"; (e.currentTarget as HTMLElement).style.color = T.textMuted; } }}
      >
        <Icon size={15} />
        <span className="flex-1 text-left">{label}</span>
        {count !== undefined && (
          <span className="text-xs px-1.5 py-0.5 rounded-full font-medium" style={{ background: active ? T.badgeActive : T.badge, color: active ? T.accentLight : T.textFaint }}>
            {count}
          </span>
        )}
      </button>
    );
  };

  const counts = {
    login: VAULT_ITEMS.filter(i => i.type === "login" && !i.trashed).length,
    note: VAULT_ITEMS.filter(i => i.type === "note" && !i.trashed).length,
    card: VAULT_ITEMS.filter(i => i.type === "card" && !i.trashed).length,
    identity: VAULT_ITEMS.filter(i => i.type === "identity" && !i.trashed).length,
    fav: VAULT_ITEMS.filter(i => i.favorite && !i.trashed).length,
    all: VAULT_ITEMS.filter(i => !i.trashed).length,
  };

  return (
    <aside className="w-[220px] flex-shrink-0 flex flex-col h-full transition-colors duration-200" style={{ background: T.bgSidebar, borderRight: `1px solid ${T.border}` }}>
      <div className="px-4 h-14 flex items-center gap-2.5" style={{ borderBottom: `1px solid ${T.border}` }}>
        <div className="w-7 h-7 rounded-lg bg-[#175ddc] flex items-center justify-center flex-shrink-0">
          <IconShield size={15} className="text-white" />
        </div>
        <div>
          <div className="text-sm font-semibold leading-tight" style={{ color: T.textPrimary }}>LocalVault</div>
          <div className="text-[10px] leading-tight" style={{ color: T.textFaint }}>Personal</div>
        </div>
      </div>

      <nav className="flex-1 overflow-y-auto p-2 space-y-0.5">
        <div className="px-2 pt-1 pb-1">
          <p className="text-[10px] font-semibold uppercase tracking-widest" style={{ color: T.sectionLabel }}>Vault</p>
        </div>
        {navItem("all", "All Items", IconKey, counts.all)}
        {navItem("favorites", "Favorites", IconStar, counts.fav)}

        <div className="px-2 pt-3 pb-1">
          <p className="text-[10px] font-semibold uppercase tracking-widest" style={{ color: T.sectionLabel }}>Types</p>
        </div>
        {navItem("logins", "Logins", IconKey, counts.login)}
        {navItem("notes", "Secure Notes", IconNote, counts.note)}
        {navItem("cards", "Payment Cards", IconCard, counts.card)}
        {navItem("identities", "Identities", IconPerson, counts.identity)}

        <div className="pt-2">
          <button
            onClick={() => setFoldersOpen(!foldersOpen)}
            className="w-full flex items-center gap-2 px-2 py-1 rounded text-[10px] font-semibold uppercase tracking-widest transition-colors"
            style={{ color: T.sectionLabel }}
          >
            <IconChevron size={10} className={`transition-transform duration-200 ${foldersOpen ? "rotate-90" : ""}`} />
            Folders
          </button>
          {foldersOpen && (
            <div className="mt-0.5 space-y-0.5">
              {FOLDERS.map(f => navItem(`folder:${f}`, f, IconFolder))}
            </div>
          )}
        </div>

        <div className="pt-2">
          <div className="px-2 pb-1">
            <p className="text-[10px] font-semibold uppercase tracking-widest" style={{ color: T.sectionLabel }}>Other</p>
          </div>
          {navItem("health", "Health Dashboard", IconActivity)}
          {navItem("generator", "Password Generator", IconZap)}
          {navItem("data", "Data Hub", IconDatabase)}
          {navItem("trash", "Trash", IconTrash, VAULT_ITEMS.filter(i => i.trashed).length)}
        </div>
      </nav>

      <div className="p-2 flex items-center gap-1" style={{ borderTop: `1px solid ${T.border}` }}>
        <button
          onClick={onLock}
          className="flex items-center justify-center w-8 h-8 rounded-lg transition-all hover:opacity-80"
          style={{ color: T.textFaint }}
          title="Lock Vault"
        >
          <IconLock size={13} />
        </button>
        <button
          onClick={toggle}
          title={dark ? "Switch to light mode" : "Switch to dark mode"}
          className="flex items-center justify-center w-8 h-8 rounded-lg transition-all hover:opacity-80"
          style={{ color: T.textFaint }}
        >
          {dark ? <IconSun size={13} /> : <IconMoon size={13} />}
        </button>
        <button
          onClick={() => setActiveSection("settings")}
          className="flex items-center justify-center w-8 h-8 rounded-lg transition-all hover:opacity-80"
          style={{ color: activeSection === "settings" ? T.accentLight : T.textFaint }}
          title="Settings"
        >
          <IconSettings size={13} />
        </button>
      </div>
    </aside>
  );
}

// ── Item row ───────────────────────────────────────────────────────────────
function ItemRow({ item, selected, onClick }: { item: VaultItem; selected: boolean; onClick: () => void }) {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  const cfg = itemTypeConfig(item.type);
  const Icon = cfg.icon;
  const credCount = item.credentials?.length ?? 0;

  const subtitle =
    item.type === "login" ? (credCount > 1 ? `${credCount} credentials` : item.username) :
    item.type === "note" ? "Secure Note" :
    item.type === "card" ? item.cardNumber :
    item.username ?? "Identity";

  return (
    <button
      onClick={onClick}
      style={{ background: selected ? T.itemSelected : "transparent", borderLeft: `2px solid ${selected ? T.itemSelectedBorder : "transparent"}`, borderBottom: `1px solid ${T.border}` }}
      className="w-full flex items-center gap-3 px-3 py-2.5 transition-all duration-100"
      onMouseEnter={e => { if (!selected) (e.currentTarget as HTMLElement).style.background = T.itemHover; }}
      onMouseLeave={e => { if (!selected) (e.currentTarget as HTMLElement).style.background = "transparent"; }}
    >
      <div className={`w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 ${cfg.bg}`}>
        <Icon size={14} className={cfg.color} />
      </div>
      <div className="flex-1 min-w-0 text-left">
        <div className="text-sm font-medium truncate" style={{ color: selected ? T.textPrimary : T.textSecondary }}>{item.name}</div>
        <div className="text-xs truncate" style={{ color: T.textFaint }}>{subtitle}</div>
      </div>
      {item.favorite && <IconStar size={11} filled className="text-[#f7b731] flex-shrink-0 opacity-70" />}
      {item.trashed && <span className="text-[10px] px-1.5 py-0.5 rounded font-medium" style={{ background: "rgba(248,78,78,0.1)", color: "#f84e4e" }}>Deleted</span>}
    </button>
  );
}

// ── Item list ──────────────────────────────────────────────────────────────
function ItemList({ items, selectedId, onSelect, section, onAdd }: { items: VaultItem[]; selectedId: string | null; onSelect: (id: string) => void; section: NavSection; onAdd?: () => void; }) {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  const [search, setSearch] = useState("");

  const sectionLabel: Record<string, string> = {
    all: "All Items", favorites: "Favorites", logins: "Logins",
    notes: "Secure Notes", cards: "Payment Cards", identities: "Identities", trash: "Trash",
  };
  const label = section.startsWith("folder:") ? section.replace("folder:", "") : (sectionLabel[section] ?? section);

  const filtered = items.filter(i =>
    i.name.toLowerCase().includes(search.toLowerCase()) ||
    i.username?.toLowerCase().includes(search.toLowerCase()) ||
    i.website?.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className="w-[280px] flex-shrink-0 flex flex-col h-full transition-colors duration-200" style={{ background: T.bgPanel, borderRight: `1px solid ${T.border}` }}>
      <div className="h-14 px-3 flex items-center justify-between" style={{ borderBottom: `1px solid ${T.border}` }}>
        <h2 className="text-sm font-semibold" style={{ color: T.textPrimary }}>{label}</h2>
        <div className="flex items-center gap-1">
          <span className="text-xs mr-1" style={{ color: T.textFaint }}>{filtered.length}</span>
          {section !== "trash" && (
            <button onClick={onAdd} className="w-7 h-7 flex items-center justify-center rounded-lg bg-[#175ddc] hover:bg-[#1e6af5] transition-colors">
              <IconPlus size={13} className="text-white" />
            </button>
          )}
        </div>
      </div>
      <div className="px-3 py-2.5" style={{ borderBottom: `1px solid ${T.border}` }}>
        <div className="relative">
          <IconSearch size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2" style={{ color: T.textFaint }} />
          <input
            type="text"
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="Search…"
            style={{ background: T.bgInput, borderColor: T.border, color: T.textSecondary }}
            className="w-full pl-8 pr-3 py-1.5 text-xs border rounded-lg focus:outline-none focus:ring-1 focus:ring-[rgba(23,93,220,0.3)] transition-all"
          />
        </div>
      </div>
      <div className="flex-1 overflow-y-auto">
        {filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-2 p-6">
            <IconSearch size={24} style={{ color: T.textFaint }} />
            <p className="text-xs text-center" style={{ color: T.textFaint }}>
              {search ? "No items match your search" : "No items here yet"}
            </p>
          </div>
        ) : (
          filtered.map(item => (
            <ItemRow key={item.id} item={item} selected={selectedId === item.id} onClick={() => onSelect(item.id)} />
          ))
        )}
      </div>
    </div>
  );
}

// ── Detail panel ───────────────────────────────────────────────────────────
function DetailPanel({ item, onEdit }: { item: VaultItem | null; onEdit?: (item: VaultItem) => void }) {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  const toast = useToast();
  const [activeTab, setActiveTab] = useState<"details" | "history">("details");
  const [confirm, setConfirm] = useState<{ open: boolean; title: string; body: string; onConfirm: () => void }>({ open: false, title: "", body: "", onConfirm: () => {} });

  if (!item) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 transition-colors duration-200" style={{ background: T.bgBase }}>
        <div className="w-16 h-16 rounded-2xl flex items-center justify-center" style={{ background: T.accentMuted }}>
          <IconShield size={28} className="text-[#175ddc] opacity-60" />
        </div>
        <div className="text-center">
          <p className="text-sm font-medium" style={{ color: T.textFaint }}>Select an item to view details</p>
          <p className="text-xs mt-0.5" style={{ color: T.textFaint, opacity: 0.6 }}>Your vault is encrypted locally</p>
        </div>
      </div>
    );
  }

  const cfg = itemTypeConfig(item.type);
  const Icon = cfg.icon;

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden transition-colors duration-200" style={{ background: T.bgBase }}>
      <ConfirmDialog
        open={confirm.open}
        title={confirm.title}
        body={confirm.body}
        confirmLabel="Delete Permanently"
        onConfirm={() => { confirm.onConfirm(); setConfirm(c => ({ ...c, open: false })); }}
        onCancel={() => setConfirm(c => ({ ...c, open: false }))}
      />
      <div className="h-14 px-5 flex items-center justify-between flex-shrink-0" style={{ borderBottom: `1px solid ${T.border}` }}>
        <div className="flex items-center gap-3">
          <div className={`w-9 h-9 rounded-xl flex items-center justify-center flex-shrink-0 ${cfg.bg}`}>
            <Icon size={16} className={cfg.color} />
          </div>
          <div>
            <h1 className="text-sm font-semibold" style={{ color: T.textPrimary }}>{item.name}</h1>
            <p className="text-xs" style={{ color: T.textFaint }}>{cfg.label}</p>
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          {item.trashed ? (
            <>
              <button onClick={() => toast("Item restored", "success")} className="px-3 py-1.5 rounded-lg text-xs font-medium text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors">Restore</button>
              <button
                onClick={() => setConfirm({ open: true, title: "Delete permanently?", body: `"${item.name}" will be removed from your vault forever. This cannot be undone.`, onConfirm: () => toast("Item deleted permanently", "error") })}
                className="px-3 py-1.5 rounded-lg text-xs font-medium text-[#f84e4e] transition-all"
                style={{ background: T.badge }}
              >
                Delete Forever
              </button>
            </>
          ) : (
            <>
              <button className="p-1.5 rounded-lg transition-all" style={{ color: item.favorite ? "#f7b731" : T.textFaint }}>
                <IconStar size={14} filled={item.favorite} />
              </button>
              <button onClick={() => onEdit?.(item)} className="px-3 py-1.5 rounded-lg text-xs font-medium transition-all hover:opacity-80" style={{ color: T.textSecondary, background: T.badge }}>Edit</button>
              <button
                onClick={() => setConfirm({ open: true, title: "Move to Trash?", body: `"${item.name}" will be moved to the Trash.`, onConfirm: () => toast("Moved to Trash") })}
                className="w-7 h-7 flex items-center justify-center rounded-lg text-[#f84e4e] hover:bg-[rgba(248,78,78,0.1)] transition-all"
              >
                <IconTrash size={14} />
              </button>
            </>
          )}
        </div>
      </div>

      <div className="flex items-center px-5 pt-4 gap-4 flex-shrink-0">
        {(["details", "history"] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className="pb-2 text-xs font-medium capitalize transition-all border-b-2"
            style={{ color: activeTab === tab ? T.accentLight : T.textFaint, borderBottomColor: activeTab === tab ? T.accent : "transparent" }}
          >
            {tab}
          </button>
        ))}
      </div>
      <div className="h-px mx-5 flex-shrink-0" style={{ background: T.border }} />

      <div className="flex-1 overflow-y-auto px-5 py-4">
        {activeTab === "details" ? (
          <div className="space-y-5">
            {item.type === "login" && (
              <>
                {item.reprompt && (
                  <div className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg w-max text-[10px] uppercase font-bold tracking-wide" style={{ background: "rgba(23,93,220,0.1)", color: "#5b9cf6", border: "1px solid rgba(23,93,220,0.2)" }}>
                    <IconLock size={10} /> Protected — requires master password
                  </div>
                )}

                {(!item.credentials || item.credentials.length === 0) && (
                  <div className="p-3 rounded-xl border space-y-3" style={{ borderColor: T.border, background: T.bgCard }}>
                    {item.username && <DetailField label="Username / Email" value={item.username} />}
                    {item.username && item.password && <div className="border-t" style={{ borderColor: T.border }} />}
                    {item.password && <DetailField label="Password" value={item.password} secret mono reprompted={item.reprompt} />}
                    {item.totp && (
                      <>
                        <div className="border-t" style={{ borderColor: T.border }} />
                        <div>
                          <p className="text-[10px] font-medium uppercase tracking-widest mb-1" style={{ color: T.textFaint }}>Authenticator Key (TOTP)</p>
                          <div className="px-3 py-2 rounded-lg" style={{ background: T.accentMuted, border: "1px solid rgba(23,93,220,0.2)" }}>
                            <p className="text-xs font-medium" style={{ color: T.accentLight, fontFamily: "'JetBrains Mono', monospace" }}>TOTP key configured</p>
                            <p className="text-[10px] mt-0.5" style={{ color: T.textFaint }}>Use your authenticator app to generate the 6-digit code</p>
                          </div>
                        </div>
                      </>
                    )}
                    {item.customFields?.map((field, idx) => (
                      <div key={idx}>
                        <div className="border-t" style={{ borderColor: T.border }} />
                        <DetailField label={field.label} value={field.value} secret={field.fieldType === "hidden"} mono={field.fieldType === "hidden"} />
                      </div>
                    ))}
                  </div>
                )}

                {item.credentials?.map((cred, idx) => (
                  <div key={cred.id} className="rounded-xl border overflow-hidden" style={{ borderColor: T.border, background: T.bgCard }}>
                    <div className="flex items-center justify-between px-3 py-2" style={{ borderBottom: `1px solid ${T.border}`, background: dark ? "rgba(255,255,255,0.02)" : "rgba(0,0,0,0.02)" }}>
                      <span className="text-[10px] font-semibold uppercase tracking-widest" style={{ color: T.textFaint }}>Credential {idx + 1}</span>
                      {cred.reprompt && (
                        <span className="flex items-center gap-1 text-[10px] font-bold uppercase" style={{ color: "#5b9cf6" }}>
                          <IconLock size={9} /> Protected
                        </span>
                      )}
                    </div>
                    <div className="p-3 space-y-3">
                      {cred.username && <DetailField label="Username / Email" value={cred.username} />}
                      {cred.password && (
                        <>
                          {cred.username && <div className="border-t" style={{ borderColor: T.border }} />}
                          <DetailField label="Password" value={cred.password} secret mono reprompted={cred.reprompt} />
                        </>
                      )}
                      {cred.totp && (
                        <>
                          <div className="border-t" style={{ borderColor: T.border }} />
                          <div>
                            <p className="text-[10px] font-medium uppercase tracking-widest mb-1" style={{ color: T.textFaint }}>Authenticator Key (TOTP)</p>
                            <div className="px-3 py-2 rounded-lg" style={{ background: T.accentMuted, border: "1px solid rgba(23,93,220,0.2)" }}>
                              <p className="text-xs font-medium" style={{ color: T.accentLight, fontFamily: "'JetBrains Mono', monospace" }}>TOTP key configured</p>
                            </div>
                          </div>
                        </>
                      )}
                      {cred.customFields?.map((field, fidx) => (
                        <div key={fidx}>
                          <div className="border-t" style={{ borderColor: T.border }} />
                          <DetailField label={field.label} value={field.value} secret={field.fieldType === "hidden"} mono={field.fieldType === "hidden"} />
                        </div>
                      ))}
                    </div>
                  </div>
                ))}

                {item.website && (
                  <div className="group p-3 rounded-xl border" style={{ borderColor: T.border, background: T.bgCard }}>
                    <p className="text-[10px] font-medium uppercase tracking-widest mb-1" style={{ color: T.textFaint }}>Website</p>
                    <div className="flex items-center gap-2">
                      <a href={item.website} target="_blank" rel="noopener noreferrer" className="flex-1 text-sm break-all hover:underline" style={{ color: T.accent }}>
                        {item.website}
                      </a>
                      <div className="opacity-0 group-hover:opacity-100 transition-opacity">
                        <CopyButton value={item.website} />
                      </div>
                    </div>
                  </div>
                )}

                {item.password && (
                  <div className="rounded-xl p-3" style={{ border: `1px solid ${T.border}`, background: T.badge }}>
                    <p className="text-[10px] font-semibold uppercase tracking-widest mb-2.5" style={{ color: T.textFaint }}>Password Health</p>
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <span className="text-xs" style={{ color: T.textMuted }}>Strength</span>
                        <div className="flex items-center gap-1.5">
                          {[1, 2, 3, 4, 5].map(i => <div key={i} className="h-1.5 w-5 rounded-full" style={{ background: i <= 4 ? "#2dcc8f" : T.border }} />)}
                          <span className="text-xs font-medium ml-1 text-[#2dcc8f]">Strong</span>
                        </div>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-xs" style={{ color: T.textMuted }}>Exposed in breaches</span>
                        <span className="text-xs font-medium text-[#2dcc8f]">None found</span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-xs" style={{ color: T.textMuted }}>Reused</span>
                        <span className="text-xs font-medium text-[#2dcc8f]">Unique</span>
                      </div>
                    </div>
                  </div>
                )}
              </>
            )}

            {item.type === "note" && item.noteContent && (
              <div>
                <p className="text-[10px] font-medium uppercase tracking-widest mb-2" style={{ color: T.textFaint }}>Note</p>
                <div className="p-3 rounded-xl" style={{ background: T.badge, border: `1px solid ${T.border}` }}>
                  <p className="text-sm leading-relaxed whitespace-pre-wrap" style={{ color: T.textSecondary }}>{item.noteContent}</p>
                </div>
              </div>
            )}

            {item.type === "card" && (
              <div className="rounded-xl border p-3 space-y-3" style={{ borderColor: T.border, background: T.bgCard }}>
                {item.cardNumber && <DetailField label="Card Number" value={item.cardNumber} mono />}
                {item.cardExpiry && <><div className="border-t" style={{ borderColor: T.border }} /><DetailField label="Expiration Date" value={item.cardExpiry} mono /></>}
                {item.cardCVV && <><div className="border-t" style={{ borderColor: T.border }} /><DetailField label="Security Code (CVV)" value={item.cardCVV} secret mono /></>}
              </div>
            )}

            {item.type === "identity" && item.username && (
              <div className="rounded-xl border p-3" style={{ borderColor: T.border, background: T.bgCard }}>
                <DetailField label="Full Name" value={item.username} />
              </div>
            )}

            <div className="pt-2 space-y-3" style={{ borderTop: `1px solid ${T.border}` }}>
              {item.folder && (
                <div>
                  <p className="text-[10px] font-medium uppercase tracking-widest mb-1" style={{ color: T.textFaint }}>Folder</p>
                  <div className="flex items-center gap-1.5">
                    <IconFolder size={12} style={{ color: T.textFaint }} />
                    <span className="text-sm" style={{ color: T.textMuted }}>{item.folder}</span>
                  </div>
                </div>
              )}
              <div className="flex gap-6">
                <div>
                  <p className="text-[10px] font-medium uppercase tracking-widest mb-1" style={{ color: T.textFaint }}>Last Modified</p>
                  <p className="text-xs" style={{ color: T.textMuted }}>{item.lastModified}</p>
                </div>
                <div>
                  <p className="text-[10px] font-medium uppercase tracking-widest mb-1" style={{ color: T.textFaint }}>Created</p>
                  <p className="text-xs" style={{ color: T.textMuted }}>{item.createdAt}</p>
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className="space-y-2">
            {[
              { action: "Password changed", date: item.lastModified, who: "You" },
              { action: "Item created", date: item.createdAt, who: "You" },
            ].map((entry, i) => (
              <div key={i} className="flex items-start gap-3 py-2" style={{ borderBottom: `1px solid ${T.border}` }}>
                <div className="w-1.5 h-1.5 rounded-full bg-[#175ddc] mt-1.5 flex-shrink-0" />
                <div>
                  <p className="text-sm" style={{ color: T.textSecondary }}>{entry.action}</p>
                  <p className="text-xs" style={{ color: T.textFaint }}>{entry.date} · {entry.who}</p>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Full-width screen header ───────────────────────────────────────────────
function ScreenHeader({ icon: Icon, title, subtitle }: { icon: React.FC<{ size?: number; className?: string }>; title: string; subtitle: string }) {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  return (
    <div className="h-14 px-6 flex items-center gap-4 flex-shrink-0" style={{ borderBottom: `1px solid ${T.border}` }}>
      <div className="w-9 h-9 rounded-xl flex items-center justify-center flex-shrink-0 bg-[#175ddc]/10 text-[#175ddc]">
        <Icon size={18} />
      </div>
      <div>
        <h1 className="text-sm font-bold" style={{ color: T.textPrimary }}>{title}</h1>
        <p className="text-xs" style={{ color: T.textMuted }}>{subtitle}</p>
      </div>
    </div>
  );
}

// ── Health Dashboard ───────────────────────────────────────────────────────
function HealthDashboard() {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  const [expanded, setExpanded] = useState<string | null>(null);
  const [breachEnabled, setBreachEnabled] = useState(false);

  const activeItems = VAULT_ITEMS.filter(i => !i.trashed && i.type === "login");

  const daysSince = (date: string) => Math.floor((Date.now() - new Date(date).getTime()) / 86400000);

  const categories = [
    {
      id: "weak",
      label: "Weak Passwords",
      color: "#f84e4e",
      bg: "rgba(248,78,78,0.1)",
      items: activeItems.filter(i => (i.password || i.credentials?.[0]?.password || "").length < 12),
    },
    {
      id: "reused",
      label: "Reused Passwords",
      color: "#f7b731",
      bg: "rgba(247,183,49,0.1)",
      items: activeItems.filter((item, _, arr) => {
        const pass = item.password || item.credentials?.[0]?.password;
        if (!pass) return false;
        return arr.some(other => other.id !== item.id && (other.password || other.credentials?.[0]?.password) === pass);
      }),
    },
    {
      id: "old",
      label: "Old Passwords",
      color: "#7c82a0",
      bg: "rgba(124,130,160,0.1)",
      items: activeItems.filter(i => daysSince(i.lastModified) > 365),
    },
    {
      id: "nototp",
      label: "No 2FA Configured",
      color: "#f0a056",
      bg: "rgba(240,160,86,0.1)",
      items: activeItems.filter(i => !i.totp && !i.credentials?.some(c => c.totp)),
    },
    {
      id: "breached",
      label: "Exposed in Breaches",
      color: "#f84e4e",
      bg: "rgba(248,78,78,0.1)",
      items: [],
    },
  ];

  const totalIssues = categories.slice(0, 4).reduce((s, c) => s + c.items.length, 0);

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden" style={{ background: T.bgBase }}>
      <ScreenHeader icon={IconActivity} title="Health Dashboard" subtitle="Security analysis of your vault" />
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-3xl mx-auto space-y-4">
          <div className="flex items-center justify-between mb-2">
            <div>
              <p className="text-xs" style={{ color: T.textMuted }}>
                {totalIssues === 0 ? "Your vault looks healthy." : `${totalIssues} issue${totalIssues !== 1 ? "s" : ""} found across your logins.`}
              </p>
            </div>
            <div className="flex items-center gap-1.5">
              <div className="w-2 h-2 rounded-full" style={{ background: totalIssues === 0 ? "#2dcc8f" : "#f7b731" }} />
              <span className="text-xs font-medium" style={{ color: totalIssues === 0 ? "#2dcc8f" : "#f7b731" }}>
                {totalIssues === 0 ? "All clear" : "Attention needed"}
              </span>
            </div>
          </div>

          {categories.map(cat => (
            <div key={cat.id} className="rounded-xl border overflow-hidden" style={{ borderColor: T.border, background: T.bgCard }}>
              <button
                onClick={() => cat.id === "breached" ? undefined : setExpanded(expanded === cat.id ? null : cat.id)}
                className="w-full flex items-center gap-4 px-5 py-4 transition-all"
                style={{ cursor: cat.id === "breached" ? "default" : "pointer" }}
                onMouseEnter={e => { if (cat.id !== "breached") (e.currentTarget as HTMLElement).style.background = T.itemHover; }}
                onMouseLeave={e => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
              >
                <div className="w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0" style={{ background: cat.bg }}>
                  <span className="text-base font-bold" style={{ color: cat.color }}>{cat.id === "breached" ? (breachEnabled ? "0" : "—") : cat.items.length}</span>
                </div>
                <div className="flex-1 text-left">
                  <p className="text-sm font-medium" style={{ color: T.textPrimary }}>{cat.label}</p>
                  <p className="text-xs" style={{ color: T.textMuted }}>
                    {cat.id === "breached"
                      ? (breachEnabled ? "No exposed passwords found" : "Opt-in breach checking is disabled")
                      : (cat.items.length === 0 ? "No issues found" : `${cat.items.length} login${cat.items.length !== 1 ? "s" : ""} affected`)}
                  </p>
                </div>
                {cat.id === "breached" ? (
                  <div className="flex items-center gap-2">
                    <span className="text-xs" style={{ color: T.textMuted }}>Enable</span>
                    <button
                      onClick={e => { e.stopPropagation(); setBreachEnabled(v => !v); }}
                      className="relative w-9 h-5 rounded-full transition-colors duration-200"
                      style={{ background: breachEnabled ? "#175ddc" : T.badge }}
                    >
                      <span className="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200" style={{ transform: breachEnabled ? "translateX(16px)" : "translateX(0)" }} />
                    </button>
                  </div>
                ) : cat.items.length > 0 ? (
                  <IconChevron size={14} className={`transition-transform duration-200 ${expanded === cat.id ? "rotate-90" : ""}`} style={{ color: T.textFaint }} />
                ) : (
                  <IconCheck size={14} style={{ color: "#2dcc8f" }} />
                )}
              </button>

              {cat.id === "breached" && breachEnabled && (
                <div className="px-5 pb-4">
                  <div className="rounded-lg p-3 text-xs" style={{ background: T.accentMuted, border: "1px solid rgba(23,93,220,0.2)", color: T.accentLight }}>
                    Breach check uses k-anonymity — only a partial hash is sent. No password ever leaves your device in plain text.
                  </div>
                </div>
              )}

              {cat.id !== "breached" && expanded === cat.id && cat.items.length > 0 && (
                <div style={{ borderTop: `1px solid ${T.border}` }}>
                  {cat.items.map(item => {
                    const cfg = itemTypeConfig(item.type);
                    const Icon = cfg.icon;
                    return (
                      <div key={item.id} className="flex items-center gap-3 px-5 py-3" style={{ borderBottom: `1px solid ${T.border}` }}>
                        <div className={`w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0 ${cfg.bg}`}>
                          <Icon size={13} className={cfg.color} />
                        </div>
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-medium" style={{ color: T.textSecondary }}>{item.name}</p>
                          <p className="text-xs" style={{ color: T.textFaint }}>{item.username ?? item.website ?? ""}</p>
                        </div>
                        {cat.id === "old" && (
                          <span className="text-xs" style={{ color: T.textFaint }}>{daysSince(item.lastModified)}d ago</span>
                        )}
                        <button className="text-xs px-2.5 py-1 rounded-lg font-medium transition-all" style={{ background: T.badge, color: T.textMuted }}>
                          View
                        </button>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

// ── Password Generator ─────────────────────────────────────────────────────
const WORDLIST = ["correct", "horse", "battery", "staple", "ocean", "tiger", "forest", "amber", "thunder", "silver", "radiant", "frozen", "marble", "cobalt", "velvet", "canyon", "crimson", "hollow", "pilgrim", "anchor", "mystic", "harbor", "serene", "falcon", "ember", "comet", "willow", "prism", "haven", "blaze"];

function calcStrength(pass: string): { score: number; label: string; color: string } {
  let score = 0;
  if (pass.length >= 8) score++;
  if (pass.length >= 14) score++;
  if (/[A-Z]/.test(pass)) score++;
  if (/[0-9]/.test(pass)) score++;
  if (/[^A-Za-z0-9]/.test(pass)) score++;
  const levels = [
    { score: 0, label: "None", color: T_COLORS.red },
    { score: 1, label: "Very Weak", color: T_COLORS.red },
    { score: 2, label: "Weak", color: T_COLORS.yellow },
    { score: 3, label: "Fair", color: T_COLORS.yellow },
    { score: 4, label: "Strong", color: T_COLORS.green },
    { score: 5, label: "Very Strong", color: T_COLORS.green },
  ];
  return levels[Math.min(score, 5)];
}

const T_COLORS = { red: "#f84e4e", yellow: "#f7b731", green: "#2dcc8f" };

function PasswordGenerator({ embedded = false }: { embedded?: boolean }) {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  const toast = useToast();
  type Method = "random" | "memorable" | "pattern" | "pronounceable";
  const [method, setMethod] = useState<Method>("random");
  const [password, setPassword] = useState("Tr!ck3y#P@ssw0rd$2026");
  const [length, setLength] = useState(20);
  const [opts, setOpts] = useState({ upper: true, lower: true, numbers: true, symbols: true, noAmbiguous: false });
  const [wordCount, setWordCount] = useState(4);
  const [separator, setSeparator] = useState("-");
  const [pattern, setPattern] = useState("Cvccvc-999-Cvccvc");

  const generate = useCallback(() => {
    if (method === "random") {
      let chars = "";
      if (opts.upper) chars += "ABCDEFGHJKLMNPQRSTUVWXYZ";
      if (opts.lower) chars += "abcdefghjkmnpqrstuvwxyz";
      if (opts.numbers) chars += opts.noAmbiguous ? "23456789" : "0123456789";
      if (opts.symbols) chars += "!@#$%^&*()_+-=[]{}|;:,.<>?";
      if (!chars) { setPassword("Enable at least one character set"); return; }
      let result = "";
      for (let i = 0; i < length; i++) result += chars[Math.floor(Math.random() * chars.length)];
      setPassword(result);
    } else if (method === "memorable") {
      const words = Array.from({ length: wordCount }, () => WORDLIST[Math.floor(Math.random() * WORDLIST.length)]);
      setPassword(words.join(separator || "-"));
    } else if (method === "pattern") {
      let result = "";
      for (const ch of pattern) {
        if (ch === "C") result += "BCDFGHJKLMNPQRSTVWXYZ"[Math.floor(Math.random() * 21)];
        else if (ch === "v") result += "aeiou"[Math.floor(Math.random() * 5)];
        else if (ch === "c") result += "bcdfghjklmnpqrstvwxyz"[Math.floor(Math.random() * 21)];
        else if (ch === "9") result += String(Math.floor(Math.random() * 10));
        else result += ch;
      }
      setPassword(result);
    } else {
      const cons = "bcdfghjklmnpqrstvwxyz";
      const vow = "aeiou";
      let result = "";
      for (let i = 0; i < length; i++) result += (i % 2 === 0 ? cons : vow)[Math.floor(Math.random() * (i % 2 === 0 ? cons.length : vow.length))];
      setPassword(result);
    }
  }, [method, length, opts, wordCount, separator, pattern]);

  useEffect(() => { generate(); }, [method]);

  const strength = calcStrength(password);

  const methodTab = (id: Method, label: string) => (
    <button
      key={id}
      onClick={() => setMethod(id)}
      className="px-3 py-1.5 rounded-lg text-xs font-medium transition-all"
      style={{ background: method === id ? T.accentMuted : "transparent", color: method === id ? T.accentLight : T.textMuted }}
    >
      {label}
    </button>
  );

  const toggle = (key: keyof typeof opts) => setOpts(o => ({ ...o, [key]: !o[key] }));

  const inner = (
    <div className="max-w-2xl mx-auto space-y-5">
      <div className="rounded-xl border overflow-hidden" style={{ borderColor: T.border, background: T.bgCard }}>
        <div className="px-5 py-4">
          <p className="font-mono text-2xl tracking-wider break-all select-all text-center leading-relaxed" style={{ color: T.textPrimary, fontFamily: "'JetBrains Mono', monospace" }}>
            {password}
          </p>
        </div>
        <div className="px-5 pb-3 flex items-center gap-3">
          <div className="flex items-center gap-1.5 flex-1">
            {[1, 2, 3, 4, 5].map(i => (
              <div key={i} className="h-1 flex-1 rounded-full transition-colors duration-300" style={{ background: i <= strength.score ? strength.color : T.border }} />
            ))}
            <span className="text-xs font-medium ml-2 w-20 flex-shrink-0" style={{ color: strength.color }}>{strength.label}</span>
          </div>
          <button onClick={generate} className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all" style={{ background: T.badge, color: T.textSecondary }}>
            <IconRefresh size={12} /> Regenerate
          </button>
          <CopyButton value={password} label="Copy" />
        </div>
      </div>

      <div className="rounded-xl border overflow-hidden" style={{ borderColor: T.border, background: T.bgCard }}>
        <div className="flex items-center gap-1 p-2" style={{ borderBottom: `1px solid ${T.border}` }}>
          {methodTab("random", "Random")}
          {methodTab("memorable", "Memorable")}
          {methodTab("pattern", "Pattern")}
          {methodTab("pronounceable", "Pronounceable")}
        </div>

        <div className="p-5 space-y-4">
          {method === "random" && (
            <>
              <div>
                <div className="flex justify-between mb-2">
                  <span className="text-xs font-medium" style={{ color: T.textSecondary }}>Length</span>
                  <span className="text-xs font-bold" style={{ color: T.textPrimary, fontFamily: "'JetBrains Mono', monospace" }}>{length}</span>
                </div>
                <input type="range" min="8" max="64" value={length} onChange={e => setLength(+e.target.value)} onMouseUp={generate} className="w-full accent-[#175ddc]" />
              </div>
              <div className="grid grid-cols-2 gap-3">
                {([["upper", "A–Z Uppercase"], ["lower", "a–z Lowercase"], ["numbers", "0–9 Numbers"], ["symbols", "!@# Symbols"]] as const).map(([key, label]) => (
                  <label key={key} className="flex items-center gap-2 text-sm cursor-pointer" style={{ color: T.textSecondary }}>
                    <input type="checkbox" checked={opts[key]} onChange={() => toggle(key)} className="rounded accent-[#175ddc] w-4 h-4" />
                    {label}
                  </label>
                ))}
                <label className="flex items-center gap-2 text-sm cursor-pointer col-span-2" style={{ color: T.textMuted }}>
                  <input type="checkbox" checked={opts.noAmbiguous} onChange={() => toggle("noAmbiguous")} className="rounded accent-[#175ddc] w-4 h-4" />
                  Exclude ambiguous characters (0, O, l, I)
                </label>
              </div>
            </>
          )}
          {method === "memorable" && (
            <div className="space-y-4">
              <div>
                <div className="flex justify-between mb-2">
                  <span className="text-xs font-medium" style={{ color: T.textSecondary }}>Word count</span>
                  <span className="text-xs font-bold" style={{ color: T.textPrimary, fontFamily: "'JetBrains Mono', monospace" }}>{wordCount}</span>
                </div>
                <input type="range" min="2" max="8" value={wordCount} onChange={e => setWordCount(+e.target.value)} onMouseUp={generate} className="w-full accent-[#175ddc]" />
              </div>
              <div>
                <label className="text-xs font-medium mb-1 block" style={{ color: T.textSecondary }}>Separator</label>
                <input
                  type="text"
                  value={separator}
                  onChange={e => setSeparator(e.target.value)}
                  maxLength={3}
                  className="w-20 px-3 py-1.5 text-sm border rounded-lg focus:outline-none focus:ring-1 focus:ring-[rgba(23,93,220,0.3)]"
                  style={{ background: T.bgInput, borderColor: T.border, color: T.textPrimary, fontFamily: "'JetBrains Mono', monospace" }}
                />
              </div>
            </div>
          )}
          {method === "pattern" && (
            <div className="space-y-3">
              <div>
                <label className="text-xs font-medium mb-1 block" style={{ color: T.textSecondary }}>Template</label>
                <input
                  type="text"
                  value={pattern}
                  onChange={e => setPattern(e.target.value)}
                  className="w-full px-3 py-2 text-sm border rounded-lg focus:outline-none focus:ring-1 focus:ring-[rgba(23,93,220,0.3)]"
                  style={{ background: T.bgInput, borderColor: T.border, color: T.textPrimary, fontFamily: "'JetBrains Mono', monospace" }}
                />
              </div>
              <div className="text-xs space-y-1" style={{ color: T.textFaint }}>
                <p><span style={{ fontFamily: "'JetBrains Mono', monospace", color: T.textMuted }}>C</span> = uppercase consonant &nbsp; <span style={{ fontFamily: "'JetBrains Mono', monospace", color: T.textMuted }}>v</span> = lowercase vowel</p>
                <p><span style={{ fontFamily: "'JetBrains Mono', monospace", color: T.textMuted }}>c</span> = lowercase consonant &nbsp; <span style={{ fontFamily: "'JetBrains Mono', monospace", color: T.textMuted }}>9</span> = digit</p>
              </div>
            </div>
          )}
          {method === "pronounceable" && (
            <div>
              <div className="flex justify-between mb-2">
                <span className="text-xs font-medium" style={{ color: T.textSecondary }}>Length</span>
                <span className="text-xs font-bold" style={{ color: T.textPrimary, fontFamily: "'JetBrains Mono', monospace" }}>{length}</span>
              </div>
              <input type="range" min="6" max="32" value={length} onChange={e => setLength(+e.target.value)} onMouseUp={generate} className="w-full accent-[#175ddc]" />
            </div>
          )}
        </div>
      </div>
    </div>
  );

  if (embedded) return inner;

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden" style={{ background: T.bgBase }}>
      <ScreenHeader icon={IconZap} title="Password Generator" subtitle="Generate strong, secure passwords" />
      <div className="flex-1 overflow-y-auto p-6">{inner}</div>
    </div>
  );
}

// ── Data Hub ───────────────────────────────────────────────────────────────
function DataHub() {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  const toast = useToast();
  type Flow = "backup" | "restore" | "export" | "import" | null;
  const [activeFlow, setActiveFlow] = useState<Flow>(null);
  const [exportMode, setExportMode] = useState<"encrypted" | "protected" | "plaintext">("encrypted");
  const [importStep, setImportStep] = useState(0);
  const [importFormat, setImportFormat] = useState("chrome");
  const [confirm, setConfirm] = useState(false);

  const actions = [
    { id: "backup" as Flow, icon: IconDownload, title: "Create Backup", desc: "Save an encrypted copy of your vault to a local file", color: "#5b9cf6" },
    { id: "restore" as Flow, icon: IconUpload, title: "Restore Backup", desc: "Replace your current vault with a previous backup", color: "#f0a056" },
    { id: "export" as Flow, icon: IconDatabase, title: "Export CSV", desc: "Export your vault credentials as a CSV file", color: "#7ec87e" },
    { id: "import" as Flow, icon: IconPlus, title: "Import CSV", desc: "Import credentials from a browser or password manager", color: "#b07ef0" },
  ];

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden" style={{ background: T.bgBase }}>
      <ConfirmDialog
        open={confirm}
        title="Replace your vault?"
        body="This will permanently overwrite your current vault with the backup. All current entries will be lost. This cannot be undone."
        confirmLabel="Replace Vault"
        onConfirm={() => { setConfirm(false); setActiveFlow(null); toast("Vault restored from backup", "success"); }}
        onCancel={() => setConfirm(false)}
      />
      <ScreenHeader icon={IconDatabase} title="Data Hub" subtitle="Backup, export, and import your vault" />
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-3xl mx-auto space-y-4">
          <div className="grid grid-cols-2 gap-3">
            {actions.map(action => {
              const Icon = action.icon;
              const isActive = activeFlow === action.id;
              return (
                <button
                  key={action.id}
                  onClick={() => setActiveFlow(isActive ? null : action.id)}
                  className="p-4 rounded-xl border text-left transition-all flex items-start gap-3"
                  style={{
                    borderColor: isActive ? `${action.color}40` : T.border,
                    background: isActive ? `${action.color}0f` : T.bgCard,
                  }}
                >
                  <div className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 mt-0.5" style={{ background: `${action.color}15`, color: action.color }}>
                    <Icon size={15} />
                  </div>
                  <div>
                    <p className="text-sm font-semibold" style={{ color: T.textPrimary }}>{action.title}</p>
                    <p className="text-xs mt-0.5" style={{ color: T.textMuted }}>{action.desc}</p>
                  </div>
                </button>
              );
            })}
          </div>

          {activeFlow === "backup" && (
            <div className="rounded-xl border p-5 space-y-4" style={{ borderColor: T.border, background: T.bgCard }}>
              <h3 className="text-sm font-semibold" style={{ color: T.textPrimary }}>Create Encrypted Backup</h3>
              <p className="text-xs" style={{ color: T.textMuted }}>Your backup will be encrypted with your master password. Keep it safe — without your master password, the backup cannot be opened.</p>
              <div className="flex items-center gap-2 px-3 py-2.5 rounded-lg" style={{ background: T.bgInput, border: `1px solid ${T.border}` }}>
                <IconFolder size={13} style={{ color: T.textFaint }} />
                <span className="text-xs flex-1" style={{ color: T.textFaint }}>~/Documents/LocalVault-backup-2026-09-20.lv</span>
                <button className="text-xs px-2 py-1 rounded" style={{ background: T.badge, color: T.textMuted }}>Change</button>
              </div>
              <button onClick={() => { toast("Backup saved to ~/Documents", "success"); setActiveFlow(null); }} className="px-4 py-2 rounded-lg text-xs font-semibold text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors">
                Save Backup
              </button>
            </div>
          )}

          {activeFlow === "restore" && (
            <div className="rounded-xl border p-5 space-y-4" style={{ borderColor: T.border, background: T.bgCard }}>
              <h3 className="text-sm font-semibold" style={{ color: T.textPrimary }}>Restore from Backup</h3>
              <div className="flex items-center gap-2 px-3 py-2.5 rounded-lg border" style={{ background: T.bgInput, borderColor: T.border }}>
                <IconUpload size={13} style={{ color: T.textFaint }} />
                <span className="text-xs flex-1" style={{ color: T.textFaint }}>No file selected</span>
                <button className="text-xs px-2 py-1 rounded" style={{ background: T.badge, color: T.textMuted }}>Browse…</button>
              </div>
              <div className="flex items-start gap-2 px-3 py-2.5 rounded-lg" style={{ background: "rgba(248,78,78,0.08)", border: "1px solid rgba(248,78,78,0.2)" }}>
                <IconAlertTriangle size={14} className="text-[#f84e4e] flex-shrink-0 mt-0.5" />
                <p className="text-xs" style={{ color: "#f84e4e" }}>Restoring a backup replaces your entire current vault. This action cannot be undone.</p>
              </div>
              <button onClick={() => setConfirm(true)} className="px-4 py-2 rounded-lg text-xs font-semibold text-white bg-[#f84e4e] hover:bg-[#d93e3e] transition-colors">
                Restore Vault
              </button>
            </div>
          )}

          {activeFlow === "export" && (
            <div className="rounded-xl border p-5 space-y-4" style={{ borderColor: T.border, background: T.bgCard }}>
              <h3 className="text-sm font-semibold" style={{ color: T.textPrimary }}>Export CSV</h3>
              <div className="space-y-2">
                {([["encrypted", "Encrypted export", "Protected with your master password — only readable in LocalVault"], ["protected", "Password-protected", "Encrypted with a custom password you set for this file"], ["plaintext", "Plaintext CSV", "Exports all passwords in plain text — readable by anyone"]] as const).map(([id, label, desc]) => (
                  <label key={id} className="flex items-start gap-3 p-3 rounded-xl cursor-pointer border transition-all" style={{ borderColor: exportMode === id ? "rgba(23,93,220,0.4)" : T.border, background: exportMode === id ? T.accentMuted : "transparent" }}>
                    <input type="radio" name="exportMode" checked={exportMode === id} onChange={() => setExportMode(id)} className="mt-0.5 accent-[#175ddc]" />
                    <div>
                      <p className="text-sm font-medium" style={{ color: T.textPrimary }}>{label}</p>
                      <p className="text-xs" style={{ color: T.textMuted }}>{desc}</p>
                    </div>
                  </label>
                ))}
              </div>
              {exportMode === "plaintext" && (
                <div className="flex items-start gap-2 px-3 py-2.5 rounded-lg" style={{ background: "rgba(248,78,78,0.08)", border: "1px solid rgba(248,78,78,0.2)" }}>
                  <IconAlertTriangle size={14} className="text-[#f84e4e] flex-shrink-0 mt-0.5" />
                  <p className="text-xs" style={{ color: "#f84e4e" }}>Plaintext export writes every password in the clear. Anyone with access to this file can read your credentials. Only use this if the destination app requires it.</p>
                </div>
              )}
              <button onClick={() => { toast("Export saved", "success"); setActiveFlow(null); }} className="px-4 py-2 rounded-lg text-xs font-semibold text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors">
                Export Now
              </button>
            </div>
          )}

          {activeFlow === "import" && (
            <div className="rounded-xl border p-5 space-y-5" style={{ borderColor: T.border, background: T.bgCard }}>
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-semibold" style={{ color: T.textPrimary }}>Import CSV</h3>
                <div className="flex items-center gap-2">
                  {[0, 1, 2].map(i => (
                    <div key={i} className="flex items-center gap-2">
                      <div className="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold transition-colors" style={{ background: importStep >= i ? "#175ddc" : T.badge, color: importStep >= i ? "#fff" : T.textFaint }}>
                        {i + 1}
                      </div>
                      {i < 2 && <div className="w-6 h-px" style={{ background: importStep > i ? "#175ddc" : T.border }} />}
                    </div>
                  ))}
                </div>
              </div>

              {importStep === 0 && (
                <div className="space-y-4">
                  <div>
                    <label className="text-xs font-medium mb-2 block" style={{ color: T.textSecondary }}>Source format</label>
                    <div className="grid grid-cols-2 gap-2">
                      {[["chrome", "Chrome / Chromium"], ["firefox", "Firefox"], ["lastpass", "LastPass"], ["generic", "Generic CSV"]].map(([id, label]) => (
                        <label key={id} className="flex items-center gap-2 p-2.5 rounded-lg border cursor-pointer transition-all text-xs" style={{ borderColor: importFormat === id ? "rgba(23,93,220,0.4)" : T.border, background: importFormat === id ? T.accentMuted : T.bgInput, color: importFormat === id ? T.accentLight : T.textSecondary }}>
                          <input type="radio" name="format" checked={importFormat === id} onChange={() => setImportFormat(id)} className="accent-[#175ddc]" />
                          {label}
                        </label>
                      ))}
                    </div>
                  </div>
                  <div className="flex items-center gap-2 px-3 py-2.5 rounded-lg border" style={{ background: T.bgInput, borderColor: T.border }}>
                    <IconUpload size={13} style={{ color: T.textFaint }} />
                    <span className="text-xs flex-1" style={{ color: T.textFaint }}>No file selected</span>
                    <button className="text-xs px-2 py-1 rounded" style={{ background: T.badge, color: T.textMuted }}>Browse…</button>
                  </div>
                  <button onClick={() => setImportStep(1)} className="px-4 py-2 rounded-lg text-xs font-semibold text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors">
                    Parse File
                  </button>
                </div>
              )}

              {importStep === 1 && (
                <div className="space-y-4">
                  <p className="text-xs" style={{ color: T.textMuted }}>Found 24 entries. Review before importing.</p>
                  <div className="rounded-lg overflow-hidden border" style={{ borderColor: T.border }}>
                    <div className="flex items-center gap-3 px-3 py-2 text-[10px] font-semibold uppercase tracking-wider" style={{ background: T.bgPanel, color: T.textFaint, borderBottom: `1px solid ${T.border}` }}>
                      <span className="w-32">Name</span>
                      <span className="w-40">URL</span>
                      <span className="flex-1">Username</span>
                      <span>Action</span>
                    </div>
                    {[["GitHub", "github.com", "maya@dev.io", "New"], ["Figma", "figma.com", "maya@dev.io", "Duplicate"], ["Notion", "notion.so", "maya@dev.io", "Duplicate"]].map(([name, url, user, action], i) => (
                      <div key={i} className="flex items-center gap-3 px-3 py-2 text-xs" style={{ borderBottom: `1px solid ${T.border}`, color: T.textSecondary }}>
                        <span className="w-32 font-medium" style={{ color: T.textPrimary }}>{name}</span>
                        <span className="w-40" style={{ color: T.textFaint }}>{url}</span>
                        <span className="flex-1">{user}</span>
                        <span className="px-2 py-0.5 rounded text-[10px] font-bold" style={{ background: action === "New" ? "rgba(45,204,143,0.1)" : "rgba(247,183,49,0.1)", color: action === "New" ? "#2dcc8f" : "#f7b731" }}>{action}</span>
                      </div>
                    ))}
                    <div className="px-3 py-2 text-xs" style={{ color: T.textFaint }}>+ 21 more entries…</div>
                  </div>
                  <div className="flex gap-2">
                    <button onClick={() => setImportStep(0)} className="px-4 py-2 rounded-lg text-xs font-medium" style={{ background: T.badge, color: T.textSecondary }}>Back</button>
                    <button onClick={() => setImportStep(2)} className="px-4 py-2 rounded-lg text-xs font-semibold text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors">Import 24 Entries</button>
                  </div>
                </div>
              )}

              {importStep === 2 && (
                <div className="space-y-4">
                  <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-full flex items-center justify-center bg-[rgba(45,204,143,0.15)]">
                      <IconCheck size={18} className="text-[#2dcc8f]" />
                    </div>
                    <div>
                      <p className="text-sm font-semibold" style={{ color: T.textPrimary }}>Import complete</p>
                      <p className="text-xs" style={{ color: T.textMuted }}>22 imported · 2 skipped (duplicates)</p>
                    </div>
                  </div>
                  <button onClick={() => { setActiveFlow(null); setImportStep(0); }} className="px-4 py-2 rounded-lg text-xs font-semibold text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors">
                    Done
                  </button>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Settings ───────────────────────────────────────────────────────────────
function SettingsToggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      onClick={() => onChange(!checked)}
      className="relative w-9 h-5 rounded-full transition-colors duration-200 flex-shrink-0"
      style={{ background: checked ? "#175ddc" : "rgba(124,130,160,0.3)" }}
    >
      <span className="absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform duration-200" style={{ transform: checked ? "translateX(16px)" : "translateX(0)" }} />
    </button>
  );
}

function SettingsScreen() {
  const { dark, toggle } = useTheme();
  const T = dark ? DARK : LIGHT;
  const toast = useToast();
  const [open, setOpen] = useState<string>("security");
  const [autoLock, setAutoLock] = useState("5");
  const [ageThreshold, setAgeThreshold] = useState("365");
  const [breachCheck, setBreachCheck] = useState(false);
  const [pinEnabled, setPinEnabled] = useState(false);
  const [biometric, setBiometric] = useState(false);
  const [sshEnabled, setSshEnabled] = useState(false);
  const [repromptMode, setRepromptMode] = useState("per-session");
  const [pwAlgo, setPwAlgo] = useState("zxcvbn");

  const sshKeys = [
    { id: "k1", label: "Work MacBook", pubkey: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA..." },
    { id: "k2", label: "Personal Desktop", pubkey: "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAAB..." },
  ];

  const section = (id: string, icon: React.FC<{ size?: number; className?: string }>, title: string, children: React.ReactNode) => {
    const Icon = icon;
    const isOpen = open === id;
    return (
      <div className="rounded-xl border overflow-hidden" style={{ borderColor: T.border, background: T.bgCard }}>
        <button
          onClick={() => setOpen(isOpen ? "" : id)}
          className="w-full flex items-center gap-3 px-5 py-4 transition-all"
          onMouseEnter={e => { (e.currentTarget as HTMLElement).style.background = T.itemHover; }}
          onMouseLeave={e => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
        >
          <div className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 bg-[#175ddc]/10 text-[#175ddc]">
            <Icon size={15} />
          </div>
          <span className="flex-1 text-left text-sm font-semibold" style={{ color: T.textPrimary }}>{title}</span>
          <IconChevron size={14} className={`transition-transform duration-200 ${isOpen ? "rotate-90" : ""}`} style={{ color: T.textFaint }} />
        </button>
        {isOpen && (
          <div className="px-5 pb-5 space-y-5" style={{ borderTop: `1px solid ${T.border}` }}>
            <div className="pt-4 space-y-5">{children}</div>
          </div>
        )}
      </div>
    );
  };

  const field = (label: string, desc: string, control: React.ReactNode) => (
    <div className="flex items-start justify-between gap-4">
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium" style={{ color: T.textPrimary }}>{label}</p>
        <p className="text-xs mt-0.5" style={{ color: T.textMuted }}>{desc}</p>
      </div>
      <div className="flex-shrink-0">{control}</div>
    </div>
  );

  const divider = () => <div className="border-t" style={{ borderColor: T.border }} />;

  const select = (value: string, onChange: (v: string) => void, options: [string, string][]) => (
    <select
      value={value}
      onChange={e => onChange(e.target.value)}
      className="text-xs border rounded-lg px-2.5 py-1.5 focus:outline-none focus:ring-1 focus:ring-[rgba(23,93,220,0.3)]"
      style={{ background: T.bgInput, borderColor: T.border, color: T.textSecondary }}
    >
      {options.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
    </select>
  );

  return (
    <div className="flex-1 flex flex-col h-full overflow-hidden" style={{ background: T.bgBase }}>
      <ScreenHeader icon={IconSettings} title="Settings" subtitle="Preferences and security configuration" />
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-2xl mx-auto space-y-3">

          {section("security", IconLock, "Security", <>
            {field("Auto-lock timeout", "Lock the vault after this period of inactivity",
              select(autoLock, setAutoLock, [["1", "1 minute"], ["5", "5 minutes"], ["15", "15 minutes"], ["30", "30 minutes"], ["never", "Never"]])
            )}
            {divider()}
            {field("Password age threshold", "Flag passwords older than this as outdated",
              select(ageThreshold, setAgeThreshold, [["90", "90 days"], ["180", "180 days"], ["365", "1 year"], ["730", "2 years"]])
            )}
            {divider()}
            {field("Breach check", "Opt in to privacy-preserving breach lookups (makes a network request)",
              <SettingsToggle checked={breachCheck} onChange={setBreachCheck} />
            )}
            {breachCheck && (
              <div className="rounded-lg px-3 py-2 text-xs" style={{ background: T.accentMuted, border: "1px solid rgba(23,93,220,0.2)", color: T.accentLight }}>
                Only a partial hash (k-anonymity) is sent — your passwords never leave your device in plain text.
              </div>
            )}
            {divider()}
            {field("PIN quick-unlock", "Unlock with a short PIN instead of the master password",
              <SettingsToggle checked={pinEnabled} onChange={v => { setPinEnabled(v); if (v) toast("PIN setup not available in this preview"); }} />
            )}
            {divider()}
            {field("Windows Hello / biometric", "Use biometric authentication when available",
              <div className="flex items-center gap-2">
                <span className="text-xs px-2 py-0.5 rounded" style={{ background: "rgba(124,130,160,0.1)", color: T.textFaint }}>Desktop only</span>
                <SettingsToggle checked={biometric} onChange={setBiometric} />
              </div>
            )}
          </>)}

          {section("ssh", IconTerminal, "SSH Agent", <>
            {field("Enable SSH agent", "Allow LocalVault to serve keys to SSH clients",
              <SettingsToggle checked={sshEnabled} onChange={setSshEnabled} />
            )}
            {sshEnabled ? (
              <>
                {divider()}
                <div>
                  <p className="text-xs font-semibold uppercase tracking-widest mb-3" style={{ color: T.textFaint }}>Stored Keys</p>
                  <div className="space-y-2">
                    {sshKeys.map(key => (
                      <div key={key.id} className="flex items-center gap-3 p-3 rounded-xl border" style={{ borderColor: T.border, background: T.bgPanel }}>
                        <div className="w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0 bg-[#175ddc]/10 text-[#175ddc]">
                          <IconKey2 size={13} />
                        </div>
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-medium" style={{ color: T.textPrimary }}>{key.label}</p>
                          <p className="text-xs truncate" style={{ color: T.textFaint, fontFamily: "'JetBrains Mono', monospace" }}>{key.pubkey}</p>
                        </div>
                        <button className="w-7 h-7 flex items-center justify-center rounded-lg text-[#f84e4e] hover:bg-[rgba(248,78,78,0.1)] transition-all">
                          <IconTrash size={13} />
                        </button>
                      </div>
                    ))}
                  </div>
                  <button onClick={() => toast("Key import not available in this preview")} className="mt-3 flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-medium transition-all" style={{ background: T.badge, color: T.textSecondary }}>
                    <IconPlus size={12} /> Add Key
                  </button>
                </div>
              </>
            ) : (
              <div className="rounded-xl p-4 text-center" style={{ background: T.bgPanel, border: `1px solid ${T.border}` }}>
                <p className="text-sm" style={{ color: T.textFaint }}>Enable the SSH agent to manage keys</p>
              </div>
            )}
          </>)}

          {section("appearance", IconPalette, "Appearance", <>
            {field("Theme", "Switch between light and dark mode",
              <div className="flex items-center gap-2">
                <span className="text-xs" style={{ color: T.textMuted }}>{dark ? "Dark" : "Light"}</span>
                <SettingsToggle checked={dark} onChange={toggle} />
              </div>
            )}
            {divider()}
            {field("Accent colour", "Primary interactive colour",
              <div className="flex items-center gap-2">
                {["#175ddc", "#7c3aed", "#0d9488", "#d97706", "#dc2626"].map(c => (
                  <button key={c} className="w-5 h-5 rounded-full border-2 transition-all" style={{ background: c, borderColor: c === "#175ddc" ? "white" : "transparent" }} />
                ))}
              </div>
            )}
          </>)}

          {section("interface", IconMonitor, "Interface", <>
            {field("UI variant", "Choose the interface style",
              <div className="flex gap-2">
                {[["classic", "Classic"], ["new", "New"]].map(([v, l]) => (
                  <label key={v} className="flex items-center gap-1.5 text-xs cursor-pointer px-3 py-1.5 rounded-lg border transition-all" style={{ borderColor: T.border, background: T.bgInput, color: T.textSecondary }}>
                    <input type="radio" name="variant" value={v} defaultChecked={v === "new"} className="accent-[#175ddc]" />
                    {l}
                  </label>
                ))}
              </div>
            )}
          </>)}

          {section("vault", IconDatabase, "Vault", <>
            {field("Password strength algorithm", "Algorithm used for strength scoring",
              select(pwAlgo, setPwAlgo, [["zxcvbn", "zxcvbn (recommended)"], ["entropy", "Entropy bits"]])
            )}
            {divider()}
            {field("Reprompt mode", "When to re-ask for the master password",
              select(repromptMode, setRepromptMode, [["per-session", "Once per session"], ["always", "Every time"], ["timed", "Every 10 minutes"]])
            )}
            {divider()}
            <div className="pt-1">
              <p className="text-xs font-semibold uppercase tracking-widest mb-3" style={{ color: T.textFaint }}>Danger Zone</p>
              <button onClick={() => toast("Not available in this preview", "error")} className="px-4 py-2 rounded-lg text-xs font-semibold text-[#f84e4e] border border-[rgba(248,78,78,0.3)] hover:bg-[rgba(248,78,78,0.08)] transition-all">
                Wipe Vault Data
              </button>
            </div>
          </>)}

        </div>
      </div>
    </div>
  );
}

// ── Auth Screen ────────────────────────────────────────────────────────────
function AuthScreen({ onUnlock }: { onUnlock: () => void }) {
  const { dark, toggle } = useTheme();
  const T = dark ? DARK : LIGHT;
  const [mode, setMode] = useState<"unlock" | "register">("unlock");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState("");

  const strengthScore = (() => {
    let s = 0;
    if (password.length >= 8) s++;
    if (password.length >= 14) s++;
    if (/[A-Z]/.test(password)) s++;
    if (/[0-9]/.test(password)) s++;
    if (/[^A-Za-z0-9]/.test(password)) s++;
    return s;
  })();
  const strengthLabel = ["", "Very Weak", "Weak", "Fair", "Strong", "Very Strong"][strengthScore];
  const strengthColor = strengthScore <= 2 ? "#f84e4e" : strengthScore <= 3 ? "#f7b731" : "#2dcc8f";

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!password) return;
    if (mode === "register") {
      if (password !== confirm) { setError("Passwords do not match"); return; }
      if (strengthScore < 3) { setError("Please choose a stronger master password"); return; }
    }
    onUnlock();
  };

  return (
    <div className="flex flex-col items-center justify-center w-full h-full transition-colors duration-200" style={{ background: T.bgBase }}>
      <div className="absolute top-6 right-6">
        <button onClick={toggle} className="p-2 rounded-lg" style={{ color: T.textFaint, background: T.badge }}>
          {dark ? <IconSun size={14} /> : <IconMoon size={14} />}
        </button>
      </div>

      <div className="w-full max-w-sm p-8 rounded-2xl shadow-xl flex flex-col items-center" style={{ background: T.bgSidebar, border: `1px solid ${T.border}` }}>
        <div className="w-12 h-12 rounded-xl bg-[#175ddc] flex items-center justify-center mb-6">
          <IconShield size={24} className="text-white" />
        </div>

        {mode === "unlock" ? (
          <>
            <h1 className="text-xl font-bold mb-1" style={{ color: T.textPrimary }}>Unlock LocalVault</h1>
            <p className="text-sm mb-8 text-center" style={{ color: T.textMuted }}>Enter your master password to access your encrypted vault.</p>
            <form onSubmit={handleSubmit} className="w-full space-y-4">
              <input
                type="password"
                placeholder="Master Password"
                value={password}
                onChange={e => setPassword(e.target.value)}
                className="w-full px-4 py-3 text-sm border rounded-xl focus:outline-none focus:ring-2 focus:ring-[#175ddc] transition-all"
                style={{ background: T.bgInput, borderColor: T.border, color: T.textPrimary }}
                autoFocus
              />
              <button type="submit" className="w-full py-3 rounded-xl text-sm font-semibold text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors">
                Unlock Vault
              </button>
            </form>
            <button onClick={() => setMode("register")} className="mt-6 text-xs transition-colors hover:underline" style={{ color: T.textFaint }}>
              New vault? Create one
            </button>
          </>
        ) : (
          <>
            <h1 className="text-xl font-bold mb-1" style={{ color: T.textPrimary }}>Create Your Vault</h1>
            <p className="text-sm mb-5 text-center" style={{ color: T.textMuted }}>Choose a strong master password. This is the only key to your vault.</p>

            <div className="w-full mb-5 p-4 rounded-xl border" style={{ background: "rgba(248,78,78,0.06)", borderColor: "rgba(248,78,78,0.2)" }}>
              <div className="flex items-start gap-2">
                <IconAlertTriangle size={14} className="text-[#f84e4e] flex-shrink-0 mt-0.5" />
                <p className="text-xs leading-relaxed" style={{ color: "#f84e4e" }}>
                  <strong>There is no recovery path.</strong> If you forget your master password, your vault cannot be opened — not by you, not by anyone. Write it down and store it somewhere safe before you continue.
                </p>
              </div>
            </div>

            <form onSubmit={handleSubmit} className="w-full space-y-3">
              <div>
                <input
                  type="password"
                  placeholder="Master Password"
                  value={password}
                  onChange={e => { setPassword(e.target.value); setError(""); }}
                  className="w-full px-4 py-3 text-sm border rounded-xl focus:outline-none focus:ring-2 focus:ring-[#175ddc] transition-all"
                  style={{ background: T.bgInput, borderColor: T.border, color: T.textPrimary }}
                  autoFocus
                />
                {password && (
                  <div className="mt-2 flex items-center gap-2">
                    <div className="flex gap-1 flex-1">
                      {[1, 2, 3, 4, 5].map(i => (
                        <div key={i} className="h-1 flex-1 rounded-full transition-colors" style={{ background: i <= strengthScore ? strengthColor : T.border }} />
                      ))}
                    </div>
                    <span className="text-xs font-medium" style={{ color: strengthColor }}>{strengthLabel}</span>
                  </div>
                )}
              </div>
              <input
                type="password"
                placeholder="Confirm Master Password"
                value={confirm}
                onChange={e => { setConfirm(e.target.value); setError(""); }}
                className="w-full px-4 py-3 text-sm border rounded-xl focus:outline-none focus:ring-2 focus:ring-[#175ddc] transition-all"
                style={{ background: T.bgInput, borderColor: T.border, color: T.textPrimary }}
              />
              {error && <p className="text-xs text-[#f84e4e]">{error}</p>}
              <button type="submit" className="w-full py-3 rounded-xl text-sm font-semibold text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors">
                Create Vault
              </button>
            </form>
            <button onClick={() => { setMode("unlock"); setPassword(""); setConfirm(""); setError(""); }} className="mt-5 text-xs transition-colors hover:underline" style={{ color: T.textFaint }}>
              Already have a vault? Unlock
            </button>
          </>
        )}
      </div>
    </div>
  );
}

// ── Add / Edit modal ───────────────────────────────────────────────────────
function generateId() {
  return Math.random().toString(36).slice(2) + Date.now().toString(36);
}

interface FormCred {
  id: string; username: string; password: string; showPassword: boolean;
  totp: string; reprompt: boolean; customFields: CustomField[];
}

function initCreds(item: VaultItem | null): FormCred[] {
  if (!item) return [{ id: generateId(), username: "", password: "", showPassword: false, totp: "", reprompt: false, customFields: [] }];
  if (item.credentials?.length) {
    return item.credentials.map(c => ({ id: c.id, username: c.username ?? "", password: c.password ?? "", showPassword: false, totp: c.totp ?? "", reprompt: c.reprompt ?? false, customFields: c.customFields ?? [] }));
  }
  return [{ id: item.id + "_c", username: item.username ?? "", password: item.password ?? "", showPassword: false, totp: item.totp ?? "", reprompt: item.reprompt ?? false, customFields: item.customFields ?? [] }];
}

function CredentialFormCard({ cred, index, canRemove, onUpdate, onRemove, onAddCustomField, onUpdateCustomField, onRemoveCustomField, T }: {
  cred: FormCred; index: number; canRemove: boolean;
  onUpdate: (patch: Partial<FormCred>) => void; onRemove: () => void;
  onAddCustomField: () => void;
  onUpdateCustomField: (i: number, patch: Partial<CustomField>) => void;
  onRemoveCustomField: (i: number) => void;
  T: TMap;
}) {
  const ic = "w-full px-3 py-2 text-sm border rounded-lg focus:outline-none focus:ring-1 focus:ring-[rgba(23,93,220,0.4)] transition-all";
  const is = { background: T.bgInput, borderColor: T.border, color: T.textPrimary } as React.CSSProperties;

  return (
    <div className="rounded-xl border overflow-hidden" style={{ borderColor: T.border, background: T.bgCard }}>
      <div className="flex items-center justify-between px-4 py-2.5" style={{ borderBottom: `1px solid ${T.border}`, background: T.bgPanel }}>
        <span className="text-[10px] font-semibold uppercase tracking-widest" style={{ color: T.textFaint }}>Credential {index + 1}</span>
        {canRemove && (
          <button onClick={onRemove} className="w-6 h-6 flex items-center justify-center rounded text-[#f84e4e] hover:bg-[rgba(248,78,78,0.1)] transition-all">
            <IconTrash size={12} />
          </button>
        )}
      </div>
      <div className="p-4 space-y-3">
        <div>
          <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Username / Email</label>
          <input type="text" value={cred.username} onChange={e => onUpdate({ username: e.target.value })} placeholder="username@example.com" className={ic} style={is} />
        </div>
        <div>
          <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Password</label>
          <div className="flex gap-2">
            <div className="relative flex-1">
              <input
                type={cred.showPassword ? "text" : "password"}
                value={cred.password}
                onChange={e => onUpdate({ password: e.target.value })}
                placeholder="Password"
                className={ic}
                style={{ ...is, fontFamily: "'JetBrains Mono', monospace", paddingRight: "2.25rem" } as React.CSSProperties}
              />
              <button type="button" onClick={() => onUpdate({ showPassword: !cred.showPassword })} className="absolute right-2.5 top-1/2 -translate-y-1/2 transition-opacity" style={{ color: T.textFaint }}>
                {cred.showPassword ? <IconEyeOff size={13} /> : <IconEye size={13} />}
              </button>
            </div>
            <button
              type="button"
              title="Generate password"
              onClick={() => {
                const chars = "abcdefghjkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTVWXYZ23456789!@#$%^&*";
                let p = "";
                for (let i = 0; i < 20; i++) p += chars[Math.floor(Math.random() * chars.length)];
                onUpdate({ password: p, showPassword: true });
              }}
              className="px-3 py-2 rounded-lg text-xs border transition-all flex items-center gap-1.5 flex-shrink-0"
              style={{ background: T.badge, borderColor: T.border, color: T.textMuted }}
            >
              <IconZap size={12} /> Generate
            </button>
          </div>
        </div>
        <div>
          <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>TOTP Secret Key</label>
          <input type="text" value={cred.totp} onChange={e => onUpdate({ totp: e.target.value })} placeholder="JBSWY3DPEHPK3PXP" className={ic} style={{ ...is, fontFamily: "'JetBrains Mono', monospace" } as React.CSSProperties} />
        </div>
        <label className="flex items-center gap-2.5 cursor-pointer select-none">
          <input type="checkbox" checked={cred.reprompt} onChange={e => onUpdate({ reprompt: e.target.checked })} className="w-4 h-4 rounded accent-[#175ddc]" />
          <span className="text-xs" style={{ color: T.textSecondary }}>Require master password to reveal</span>
        </label>
        {cred.customFields.length > 0 && (
          <div className="pt-3 space-y-2" style={{ borderTop: `1px solid ${T.border}` }}>
            <p className="text-[10px] font-semibold uppercase tracking-widest mb-2" style={{ color: T.textFaint }}>Custom Fields</p>
            {cred.customFields.map((field, fidx) => (
              <div key={fidx} className="flex items-center gap-2">
                <input type="text" value={field.label} onChange={e => onUpdateCustomField(fidx, { label: e.target.value })} placeholder="Label" className={ic} style={{ ...is, flex: "0 0 130px" } as React.CSSProperties} />
                <select value={field.fieldType} onChange={e => onUpdateCustomField(fidx, { fieldType: e.target.value as CustomField["fieldType"] })} className={ic} style={{ ...is, flex: "0 0 90px" } as React.CSSProperties}>
                  <option value="text">Text</option>
                  <option value="hidden">Hidden</option>
                  <option value="boolean">Boolean</option>
                </select>
                <input type="text" value={field.value} onChange={e => onUpdateCustomField(fidx, { value: e.target.value })} placeholder="Value" className={ic} style={{ ...is, flex: 1 } as React.CSSProperties} />
                <button onClick={() => onRemoveCustomField(fidx)} className="w-8 h-[34px] flex items-center justify-center rounded-lg text-[#f84e4e] hover:bg-[rgba(248,78,78,0.1)] transition-all flex-shrink-0">
                  <IconTrash size={12} />
                </button>
              </div>
            ))}
          </div>
        )}
        <button onClick={onAddCustomField} className="flex items-center gap-1.5 text-xs transition-all hover:opacity-80 mt-1" style={{ color: T.accentLight }}>
          <IconPlus size={11} /> Add custom field
        </button>
      </div>
    </div>
  );
}

function AddEditModal({ item, defaultType = "login", onClose, onSave }: {
  item: VaultItem | null;
  defaultType?: ItemType;
  onClose: () => void;
  onSave: (item: VaultItem) => void;
}) {
  const { dark } = useTheme();
  const T = dark ? DARK : LIGHT;
  const isEdit = !!item;

  const [type, setType] = useState<ItemType>(item?.type ?? defaultType);
  const [name, setName] = useState(item?.name ?? "");
  const [folder, setFolder] = useState(item?.folder ?? "");
  const [favorite, setFavorite] = useState(item?.favorite ?? false);
  const [website, setWebsite] = useState(item?.website ?? "");
  const [noteContent, setNoteContent] = useState(item?.noteContent ?? "");
  const [cardNumber, setCardNumber] = useState(item?.cardNumber ?? "");
  const [cardExpiry, setCardExpiry] = useState(item?.cardExpiry ?? "");
  const [cardCVV, setCardCVV] = useState(item?.cardCVV ?? "");
  const [identityName, setIdentityName] = useState(item?.username ?? "");
  const [creds, setCreds] = useState<FormCred[]>(() => initCreds(item));
  const [nameError, setNameError] = useState(false);

  const updateCred = (id: string, patch: Partial<FormCred>) =>
    setCreds(cs => cs.map(c => c.id === id ? { ...c, ...patch } : c));

  const addCred = () =>
    setCreds(cs => [...cs, { id: generateId(), username: "", password: "", showPassword: false, totp: "", reprompt: false, customFields: [] }]);

  const addCustomField = (credId: string) =>
    setCreds(cs => cs.map(c => c.id === credId ? { ...c, customFields: [...c.customFields, { label: "", value: "", fieldType: "text" as const }] } : c));

  const updateCustomField = (credId: string, idx: number, patch: Partial<CustomField>) =>
    setCreds(cs => cs.map(c => c.id === credId ? { ...c, customFields: c.customFields.map((f, i) => i === idx ? { ...f, ...patch } : f) } : c));

  const removeCustomField = (credId: string, idx: number) =>
    setCreds(cs => cs.map(c => c.id === credId ? { ...c, customFields: c.customFields.filter((_, i) => i !== idx) } : c));

  const handleSave = () => {
    if (!name.trim()) { setNameError(true); return; }
    const now = new Date().toISOString().split("T")[0];
    const base: VaultItem = {
      id: item?.id ?? generateId(), type, name: name.trim(),
      folder: folder || undefined, favorite, lastModified: now, createdAt: item?.createdAt ?? now,
    };
    if (type === "login") {
      base.website = website || undefined;
      if (creds.length === 1) {
        const c = creds[0];
        base.username = c.username || undefined;
        base.password = c.password || undefined;
        base.totp = c.totp || undefined;
        base.reprompt = c.reprompt || undefined;
        base.customFields = c.customFields.length ? c.customFields : undefined;
      } else {
        base.credentials = creds.map(c => ({ id: c.id, username: c.username || undefined, password: c.password || undefined, totp: c.totp || undefined, reprompt: c.reprompt || undefined, customFields: c.customFields.length ? c.customFields : undefined }));
      }
    } else if (type === "note") {
      base.noteContent = noteContent;
    } else if (type === "card") {
      base.cardNumber = cardNumber || undefined;
      base.cardExpiry = cardExpiry || undefined;
      base.cardCVV = cardCVV || undefined;
    } else {
      base.username = identityName || undefined;
    }
    onSave(base);
  };

  const ic = "w-full px-3 py-2 text-sm border rounded-lg focus:outline-none focus:ring-1 focus:ring-[rgba(23,93,220,0.4)] transition-all";
  const is = { background: T.bgInput, borderColor: T.border, color: T.textPrimary } as React.CSSProperties;
  const typeConfig = itemTypeConfig(type);
  const TypeIcon = typeConfig.icon;

  return (
    <div
      className="fixed inset-0 z-50 flex items-stretch justify-end"
      style={{ background: "rgba(0,0,0,0.55)", backdropFilter: "blur(4px)" }}
      onClick={e => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="w-full max-w-xl flex flex-col h-full shadow-2xl" style={{ background: T.bgBase }}>
        {/* Header */}
        <div className="h-14 px-5 flex items-center justify-between flex-shrink-0" style={{ borderBottom: `1px solid ${T.border}`, background: T.bgSidebar }}>
          <div className="flex items-center gap-3">
            <div className={`w-8 h-8 rounded-lg flex items-center justify-center ${typeConfig.bg}`}>
              <TypeIcon size={14} className={typeConfig.color} />
            </div>
            <h2 className="text-sm font-semibold" style={{ color: T.textPrimary }}>
              {isEdit ? `Edit — ${item.name}` : "New Item"}
            </h2>
          </div>
          <button onClick={onClose} className="w-8 h-8 flex items-center justify-center rounded-lg transition-all text-sm hover:opacity-70" style={{ color: T.textFaint, background: T.badge }}>
            ✕
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto">
          <div className="p-5 space-y-5">

            {/* Type tabs */}
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-widest mb-2" style={{ color: T.textFaint }}>Item Type</p>
              <div className="flex gap-2 flex-wrap">
                {([["login", "Login", IconKey], ["note", "Secure Note", IconNote], ["card", "Payment Card", IconCard], ["identity", "Identity", IconPerson]] as [ItemType, string, React.FC<IconProps>][]).map(([id, label, Icon]) => (
                  <button
                    key={id}
                    onClick={() => setType(id)}
                    className="flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-medium transition-all border"
                    style={{ background: type === id ? T.accentMuted : T.bgCard, borderColor: type === id ? "rgba(23,93,220,0.4)" : T.border, color: type === id ? T.accentLight : T.textMuted }}
                  >
                    <Icon size={12} /> {label}
                  </button>
                ))}
              </div>
            </div>

            {/* Name + Folder + Favorite */}
            <div className="space-y-3">
              <div>
                <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Name *</label>
                <input
                  type="text" value={name}
                  onChange={e => { setName(e.target.value); setNameError(false); }}
                  placeholder={type === "login" ? "e.g. GitHub" : type === "note" ? "e.g. SSH Key Passphrase" : type === "card" ? "e.g. Chase Sapphire" : "e.g. Work Passport"}
                  className={ic}
                  style={{ ...is, borderColor: nameError ? "#f84e4e" : T.border } as React.CSSProperties}
                  autoFocus
                />
                {nameError && <p className="text-xs mt-1 text-[#f84e4e]">Name is required</p>}
              </div>
              <div className="flex gap-3">
                <div className="flex-1">
                  <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Folder</label>
                  <select value={folder} onChange={e => setFolder(e.target.value)} className={ic} style={is}>
                    <option value="">No folder</option>
                    {FOLDERS.map(f => <option key={f} value={f}>{f}</option>)}
                  </select>
                </div>
                <div>
                  <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Favorite</label>
                  <button
                    onClick={() => setFavorite(v => !v)}
                    className="flex items-center gap-2 px-3 py-2 rounded-lg border text-xs font-medium transition-all"
                    style={{ background: favorite ? "rgba(247,183,49,0.08)" : T.bgCard, borderColor: favorite ? "rgba(247,183,49,0.4)" : T.border, color: favorite ? "#f7b731" : T.textMuted }}
                  >
                    <IconStar size={13} filled={favorite} /> {favorite ? "Favorited" : "Add to Favorites"}
                  </button>
                </div>
              </div>
            </div>

            {/* Login fields */}
            {type === "login" && (
              <>
                <div>
                  <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Website URL</label>
                  <input type="url" value={website} onChange={e => setWebsite(e.target.value)} placeholder="https://example.com" className={ic} style={is} />
                </div>
                <div>
                  <p className="text-[10px] font-semibold uppercase tracking-widest mb-3" style={{ color: T.textFaint }}>Credentials</p>
                  <div className="space-y-3">
                    {creds.map((cred, idx) => (
                      <CredentialFormCard
                        key={cred.id} cred={cred} index={idx} canRemove={creds.length > 1} T={T}
                        onUpdate={patch => updateCred(cred.id, patch)}
                        onRemove={() => setCreds(cs => cs.filter(c => c.id !== cred.id))}
                        onAddCustomField={() => addCustomField(cred.id)}
                        onUpdateCustomField={(i, patch) => updateCustomField(cred.id, i, patch)}
                        onRemoveCustomField={i => removeCustomField(cred.id, i)}
                      />
                    ))}
                  </div>
                  <button
                    onClick={addCred}
                    className="mt-3 flex items-center gap-2 px-3 py-2.5 rounded-lg text-xs font-medium transition-all border w-full justify-center"
                    style={{ borderStyle: "dashed", borderColor: T.border, color: T.textMuted }}
                  >
                    <IconPlus size={12} /> Add Another Credential
                  </button>
                </div>
              </>
            )}

            {/* Note */}
            {type === "note" && (
              <div>
                <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Note Content</label>
                <textarea value={noteContent} onChange={e => setNoteContent(e.target.value)} placeholder="Enter your note…" rows={9} className={ic + " resize-none"} style={is} />
              </div>
            )}

            {/* Card */}
            {type === "card" && (
              <div className="space-y-3">
                <div>
                  <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Card Number</label>
                  <input type="text" value={cardNumber} onChange={e => setCardNumber(e.target.value)} placeholder="**** **** **** 0000" className={ic} style={{ ...is, fontFamily: "'JetBrains Mono', monospace" } as React.CSSProperties} />
                </div>
                <div className="flex gap-3">
                  <div className="flex-1">
                    <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Expiry</label>
                    <input type="text" value={cardExpiry} onChange={e => setCardExpiry(e.target.value)} placeholder="MM/YY" className={ic} style={{ ...is, fontFamily: "'JetBrains Mono', monospace" } as React.CSSProperties} />
                  </div>
                  <div className="flex-1">
                    <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>CVV</label>
                    <input type="text" value={cardCVV} onChange={e => setCardCVV(e.target.value)} placeholder="***" className={ic} style={{ ...is, fontFamily: "'JetBrains Mono', monospace" } as React.CSSProperties} />
                  </div>
                </div>
              </div>
            )}

            {/* Identity */}
            {type === "identity" && (
              <div>
                <label className="text-[10px] font-medium uppercase tracking-widest mb-1.5 block" style={{ color: T.textFaint }}>Full Name</label>
                <input type="text" value={identityName} onChange={e => setIdentityName(e.target.value)} placeholder="First Last" className={ic} style={is} />
              </div>
            )}

          </div>
        </div>

        {/* Footer */}
        <div className="px-5 py-4 flex items-center justify-end gap-3 flex-shrink-0" style={{ borderTop: `1px solid ${T.border}`, background: T.bgSidebar }}>
          <button onClick={onClose} className="px-4 py-2 rounded-lg text-xs font-medium transition-all hover:opacity-80" style={{ background: T.badge, color: T.textSecondary }}>
            Cancel
          </button>
          <button onClick={handleSave} className="px-5 py-2 rounded-lg text-xs font-semibold text-white bg-[#175ddc] hover:bg-[#1252b8] transition-colors">
            {isEdit ? "Save Changes" : "Add Item"}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── App ────────────────────────────────────────────────────────────────────
export default function App() {
  const [locked, setLocked] = useState(true);
  const [dark, setDark] = useState(true);
  const [activeSection, setActiveSection] = useState<NavSection>("all");
  const [selectedId, setSelectedId] = useState<string | null>(VAULT_ITEMS[0].id);
  const [foldersOpen, setFoldersOpen] = useState(true);
  const [items, setItems] = useState<VaultItem[]>(VAULT_ITEMS);
  const [editState, setEditState] = useState<{ mode: "add"; defaultType?: ItemType } | { mode: "edit"; item: VaultItem } | null>(null);

  const T = dark ? DARK : LIGHT;

  const filteredItems = items.filter(item => {
    if (activeSection === "trash") return !!item.trashed;
    if (item.trashed) return false;
    if (activeSection === "all") return true;
    if (activeSection === "favorites") return item.favorite;
    if (activeSection === "logins") return item.type === "login";
    if (activeSection === "notes") return item.type === "note";
    if (activeSection === "cards") return item.type === "card";
    if (activeSection === "identities") return item.type === "identity";
    if (activeSection.startsWith("folder:")) return item.folder === activeSection.replace("folder:", "");
    return true;
  });

  const selectedItem = items.find(i => i.id === selectedId) ?? null;

  const handleSave = (saved: VaultItem) => {
    setItems(prev => prev.some(i => i.id === saved.id) ? prev.map(i => i.id === saved.id ? saved : i) : [saved, ...prev]);
    setSelectedId(saved.id);
    setActiveSection("all");
    setEditState(null);
  };

  const isFullWidth = ["health", "generator", "data", "settings"].includes(activeSection);

  if (locked) {
    return (
      <ThemeCtx.Provider value={{ dark, toggle: () => setDark(d => !d) }}>
        <ToastProvider>
          <AuthScreen onUnlock={() => setLocked(false)} />
        </ToastProvider>
      </ThemeCtx.Provider>
    );
  }

  return (
    <ThemeCtx.Provider value={{ dark, toggle: () => setDark(d => !d) }}>
      <ToastProvider>
        <div className="flex h-full overflow-hidden transition-colors duration-200" style={{ background: T.bgBase, fontFamily: "Inter, sans-serif" }}>
          <Sidebar
            activeSection={activeSection}
            setActiveSection={s => { setActiveSection(s); setSelectedId(null); }}
            foldersOpen={foldersOpen}
            setFoldersOpen={setFoldersOpen}
            onLock={() => setLocked(true)}
          />
          {isFullWidth ? (
            <>
              {activeSection === "health" && <HealthDashboard />}
              {activeSection === "generator" && <PasswordGenerator />}
              {activeSection === "data" && <DataHub />}
              {activeSection === "settings" && <SettingsScreen />}
            </>
          ) : (
            <>
              <ItemList
                items={filteredItems}
                selectedId={selectedId}
                onSelect={setSelectedId}
                section={activeSection}
                onAdd={() => setEditState({ mode: "add" })}
              />
              <DetailPanel
                item={selectedItem}
                onEdit={item => setEditState({ mode: "edit", item })}
              />
            </>
          )}
          {editState && (
            <AddEditModal
              item={editState.mode === "edit" ? editState.item : null}
              defaultType={editState.mode === "add" ? editState.defaultType : undefined}
              onClose={() => setEditState(null)}
              onSave={handleSave}
            />
          )}
        </div>
      </ToastProvider>
    </ThemeCtx.Provider>
  );
}
