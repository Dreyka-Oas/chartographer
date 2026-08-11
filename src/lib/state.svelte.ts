import { api } from "./api";
import type { AppErrorPayload, AuthStatus, Overview, SyncReport } from "./types";

function message(e: unknown): string {
  return (e as AppErrorPayload)?.message ?? String(e);
}

class Dashboard {
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
    if (this.overview && this.overview.per_project.length === 0) await this.sync();
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
