import { createRoot } from "react-dom/client";
import { api, type ConfigDto, type DeleteRepromptAction } from "./api";
import { confirmDialog } from "./confirm";
import { MasterPasswordPrompt } from "../components/MasterPasswordPrompt";

export function deleteNeedsMasterPassword(config: ConfigDto, action: DeleteRepromptAction): boolean {
  switch (action) {
    case "trashItem":
      return config.deleteRepromptTrashItem;
    case "deleteCredential":
      return config.deleteRepromptCredential;
    case "emptyTrash":
      return config.deleteRepromptEmptyTrash;
    case "deleteFolder":
      return config.deleteRepromptFolder;
  }
}

function promptMasterPassword(title: string): Promise<boolean> {
  return new Promise((resolve) => {
    const host = document.createElement("div");
    // Own stacking context above the z-[60] drawers/modals a delete can be launched from.
    host.style.position = "relative";
    host.style.zIndex = "100";
    document.body.appendChild(host);
    const root = createRoot(host);
    root.render(
      <MasterPasswordPrompt
        title={title}
        onResult={(ok) => {
          root.unmount();
          host.remove();
          resolve(ok);
        }}
      />,
    );
  });
}

// Deliberately ignores the reveal/copy grace window and per-item exemptions: deletes re-check every time.
export async function confirmDelete(action: DeleteRepromptAction, message: string): Promise<boolean> {
  if (!(await confirmDialog(message))) return false;
  const config = await api.getConfig();
  if (!deleteNeedsMasterPassword(config, action)) return true;
  return promptMasterPassword("Confirm delete");
}
