import { invoke } from "@tauri-apps/api/core";
import type { AuthStatus, Overview, Settings, SyncReport } from "./types";

export const api = {
  authStatus: () => invoke<AuthStatus>("auth_status"),
  connect: (token: string) => invoke<AuthStatus>("connect", { token }),
  logout: () => invoke<AuthStatus>("logout"),
  openTokenPage: () => invoke<void>("open_token_page"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (curseforgeUsername: string | null, rangeDays: number) =>
    invoke<void>("save_settings", { curseforgeUsername, rangeDays }),
  syncNow: () => invoke<SyncReport[]>("sync_now"),
  overview: (rangeDays: number) => invoke<Overview>("overview", { rangeDays }),
  linkManual: (modrinthId: number, curseforgeId: number) =>
    invoke<void>("link_manual", { modrinthId, curseforgeId }),
  unlink: (modrinthId: number, curseforgeId: number) =>
    invoke<void>("unlink", { modrinthId, curseforgeId }),
  unlinkedProjects: () => invoke<[number, string, string][]>("unlinked_projects"),
};
