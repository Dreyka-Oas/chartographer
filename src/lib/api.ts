import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AuthStatus,
  CfAnalysis,
  CfCollect,
  CfGesture,
  CfPointEntry,
  CfScrape,
  CfSession,
  CurrencyView,
  DayRankings,
  DayReport,
  RankBy,
  RankScope,
  FollowersReport,
  GameVersion,
  Overview,
  PairingEntry,
  ProjectDetail,
  PublishDraft,
  PublishOutcome,
  PublishReport,
  Settings,
  SyncReport,
  SyncStep,
} from "./types";

export const api = {
  authStatus: () => invoke<AuthStatus>("auth_status"),
  connect: (token: string) => invoke<AuthStatus>("connect", { token }),
  logout: () => invoke<AuthStatus>("logout"),
  openTokenPage: () => invoke<void>("open_token_page"),
  openAccountPage: (platform: string, username: string) =>
    invoke<void>("open_account_page", { platform, username }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (
    curseforgeUsername: string | null,
    rangeDays: number,
    currency?: string,
    autoSyncMinutes?: number,
  ) => invoke<void>("save_settings", { curseforgeUsername, rangeDays, currency, autoSyncMinutes }),
  refreshExchangeRate: () => invoke<CurrencyView>("refresh_exchange_rate"),
  syncNow: () => invoke<SyncReport[]>("sync_now"),
  /** Jalons posés par le cycle en cours, étape après étape. */
  onSyncStep: (handler: (step: SyncStep) => void) =>
    listen<SyncStep>("sync:step", (event) => handler(event.payload)),
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
  openCurseforgeSite: () => invoke<void>("open_curseforge_site"),
  openCurseforgeWindow: () => invoke<void>("open_curseforge_window"),
  dayReport: (day: string | null, platforms: string[]) =>
    invoke<DayReport>("day_report", { day, platforms }),
  dayRankings: (
    rangeDays: number,
    from: string | null,
    to: string | null,
    platforms: string[],
    by: RankBy,
    scope: RankScope,
    windowDays: number | null,
  ) =>
    invoke<DayRankings>("day_rankings", { rangeDays, from, to, platforms, by, scope, windowDays }),
  curseforgeSession: () => invoke<CfSession>("curseforge_session"),
  curseforgeFollowers: () => invoke<FollowersReport>("curseforge_followers"),
  collectCurseforgeFollowers: () => invoke<FollowersReport>("collect_curseforge_followers"),
  armCurseforgeCapture: () => invoke<string>("arm_curseforge_capture"),
  collectCurseforge: () => invoke<CfCollect>("collect_curseforge"),
  readCurseforgePage: () => invoke<CfScrape>("read_curseforge_page"),
  importCurseforgeCapture: (curseforgeId: number, url: string) =>
    invoke<number>("import_curseforge_capture", { curseforgeId, url }),
  analyzeCurseforgeText: (text: string) => invoke<CfAnalysis>("analyze_curseforge_text", { text }),
  importCurseforgeSeries: (curseforgeId: number, text: string) =>
    invoke<number>("import_curseforge_series", { curseforgeId, text }),
  pairingState: () => invoke<PairingEntry[]>("pairing_state"),

  // Publication
  curseforgeGameVersions: () => invoke<GameVersion[]>("curseforge_game_versions"),
  captureCurseforgeToken: () => invoke<boolean>("capture_curseforge_token"),
  publishVersion: (draft: PublishDraft, filePath: string) =>
    invoke<PublishReport>("publish_version", { draft, filePath }),
  createModrinthProject: (
    slug: string,
    title: string,
    description: string,
    body: string,
    projectType: string,
    categories: string[],
  ) =>
    invoke<PublishOutcome>("create_modrinth_project", {
      slug,
      title,
      description,
      body,
      projectType,
      categories,
    }),
  deleteModrinthVersion: (versionId: string) =>
    invoke<PublishOutcome>("delete_modrinth_version", { versionId }),
  deleteModrinthProject: (projectId: string) =>
    invoke<PublishOutcome>("delete_modrinth_project", { projectId }),

  // Gestes CurseForge : l'application regarde faire, puis sait refaire.
  watchCurseforge: () => invoke<string>("watch_curseforge"),
  learnCurseforge: () => invoke<CfGesture[]>("learn_curseforge"),
  curseforgeGestures: () => invoke<CfGesture[]>("curseforge_gestures"),
  createCurseforgeProject: (name: string, summary: string) =>
    invoke<PublishOutcome>("create_curseforge_project", { name, summary }),
  deleteCurseforgeFile: (projectId: number, fileId: number) =>
    invoke<PublishOutcome>("delete_curseforge_file", { projectId, fileId }),
  curseforgeFiles: (projectId: number) =>
    invoke<unknown>("curseforge_files", { projectId }),
};
