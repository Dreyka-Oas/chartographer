import { invoke } from "@tauri-apps/api/core";
import type {
  AuthStatus,
  CfPointEntry,
  CfScrape,
  Overview,
  PairingEntry,
  ProjectDetail,
  Settings,
  SyncReport,
} from "./types";

export const api = {
  authStatus: () => invoke<AuthStatus>("auth_status"),
  connect: (token: string) => invoke<AuthStatus>("connect", { token }),
  logout: () => invoke<AuthStatus>("logout"),
  openTokenPage: () => invoke<void>("open_token_page"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (curseforgeUsername: string | null, rangeDays: number) =>
    invoke<void>("save_settings", { curseforgeUsername, rangeDays }),
  syncNow: () => invoke<SyncReport[]>("sync_now"),
  overview: (rangeDays: number, from: string | null, to: string | null, platforms: string[]) =>
    invoke<Overview>("overview", { rangeDays, from, to, platforms }),
  projectDetail: (
    modrinthId: number | null,
    curseforgeId: number | null,
    rangeDays: number,
    from: string | null,
    to: string | null,
  ) => invoke<ProjectDetail>("project_detail", { modrinthId, curseforgeId, rangeDays, from, to }),
  linkManual: (modrinthId: number, curseforgeId: number) =>
    invoke<void>("link_manual", { modrinthId, curseforgeId }),
  unlink: (modrinthId: number, curseforgeId: number) =>
    invoke<void>("unlink", { modrinthId, curseforgeId }),
  setSolo: (projectId: number, solo: boolean) => invoke<void>("set_solo", { projectId, solo }),
  recordCurseforgePoints: (points: number) =>
    invoke<void>("record_curseforge_points", { points }),
  forgetCurseforgePoints: (day: string) => invoke<void>("forget_curseforge_points", { day }),
  curseforgePoints: () => invoke<CfPointEntry[]>("curseforge_points"),
  openCurseforgeWindow: () => invoke<void>("open_curseforge_window"),
  readCurseforgePage: () => invoke<CfScrape>("read_curseforge_page"),
  pairingState: () => invoke<PairingEntry[]>("pairing_state"),
};
