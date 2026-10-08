import type { AppState } from "./state";

/**
 * Handle the Android back button: close menus, return to main pane, or exit.
 */
export function backStep(s: Pick<AppState, "chatMenuId" | "settingsOpen" | "pane" | "section">): Partial<AppState> | undefined {
  if (s.chatMenuId) return { chatMenuId: undefined, movingChatId: undefined };
  if (s.settingsOpen) return { settingsOpen: false };
  if (s.pane !== "main") return { pane: "main" };
  if (s.section !== "chat") return { section: "chat", pane: "main" };
  return undefined;
}
