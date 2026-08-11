import { api } from "./api";
import type {
  AppErrorPayload,
  AuthStatus,
  Overview,
  ProjectDetail,
  ProjectSummary,
  SyncReport,
} from "./types";

function message(e: unknown): string {
  return (e as AppErrorPayload)?.message ?? String(e);
}

/** Vues plein écran ouvertes depuis une carte de la page de vision. */
export type DetailView =
  | "timeline"
  | "countries"
  | "platforms"
  | "loaders"
  | "revenue"
  | "events"
  | "projects";

class Dashboard {
  detail = $state<DetailView | null>(null);
  project = $state<ProjectDetail | null>(null);
  projectLoading = $state(false);
  auth = $state<AuthStatus | null>(null);
  overview = $state<Overview | null>(null);
  rangeDays = $state(90);
  loading = $state(false);
  syncing = $state(false);
  connecting = $state(false);
  error = $state<string | null>(null);
  lastSync = $state<SyncReport[]>([]);
  selectedProject = $state<string | null>(null);

  async refreshAuth() {
    this.auth = await api.authStatus();
  }

  /**
   * Démarrage : si un token est déjà enregistré, on charge la base, et si elle
   * est encore vide on lance la première synchronisation sans rien demander.
   */
  async boot() {
    await this.refreshAuth();
    if (!this.auth?.connected) return;
    await this.load();
    if (!this.overview) return;
    // Base vide, ou échéancier de reversement jamais relevé : on synchronise.
    const stale =
      this.overview.per_project.length === 0 || this.overview.payout.available === "";
    if (stale) await this.sync();
  }

  /**
   * Valide le token côté Rust avant de l'enregistrer, puis enchaîne
   * directement sur une première synchronisation.
   */
  async connect(token: string) {
    this.connecting = true;
    this.error = null;
    try {
      this.auth = await api.connect(token);
      await this.sync();
    } catch (e) {
      this.error = message(e);
    } finally {
      this.connecting = false;
    }
  }

  async logout() {
    this.auth = await api.logout();
    this.overview = null;
  }

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.overview = await api.overview(this.rangeDays);
    } catch (e) {
      this.error = message(e);
    } finally {
      this.loading = false;
    }
  }

  async setRange(days: number) {
    this.rangeDays = days;
    await this.load();
    if (this.project) await this.openProject(this.project.summary);
  }

  openDetail(view: DetailView) {
    this.detail = view;
  }

  /** Ferme la vue plein écran courante, quelle qu'elle soit. */
  closeDetail() {
    this.detail = null;
    this.project = null;
    this.selectedProject = null;
  }

  async openProject(summary: ProjectSummary) {
    this.selectedProject = summary.key;
    this.projectLoading = true;
    this.error = null;
    try {
      this.project = await api.projectDetail(
        summary.modrinth_id,
        summary.curseforge_id,
        this.rangeDays,
      );
    } catch (e) {
      this.error = message(e);
      this.project = null;
    } finally {
      this.projectLoading = false;
    }
  }

  async sync() {
    this.syncing = true;
    this.error = null;
    try {
      this.lastSync = await api.syncNow();
      await this.load();
    } catch (e) {
      this.error = message(e);
    } finally {
      this.syncing = false;
    }
  }
}

export const dashboard = new Dashboard();
