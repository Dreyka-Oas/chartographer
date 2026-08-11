import { api } from "./api";
import { lastDayOfMonth } from "./format";
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
  | "events"
  | "projects";

/** Au-delà de ce délai, les données sont considérées périmées et resynchronisées. */
const STALE_AFTER_MS = 6 * 60 * 60 * 1000;
/** Cadence du réveil qui vérifie la péremption. */
const AUTO_SYNC_TICK_MS = 15 * 60 * 1000;

class Dashboard {
  detail = $state<DetailView | null>(null);
  project = $state<ProjectDetail | null>(null);
  projectLoading = $state(false);
  auth = $state<AuthStatus | null>(null);
  overview = $state<Overview | null>(null);
  rangeDays = $state(90);
  /** Bornes explicites. Nulles, la fenêtre glisse sur `rangeDays` jours. */
  rangeFrom = $state<string | null>(null);
  rangeTo = $state<string | null>(null);
  /** Plateformes affichées. Masquer n'efface rien : seuls les relevés lus changent. */
  platforms = $state({ modrinth: true, curseforge: true });
  loading = $state(false);
  syncing = $state(false);
  connecting = $state(false);
  error = $state<string | null>(null);
  lastSync = $state<SyncReport[]>([]);
  selectedProject = $state<string | null>(null);
  private timer: ReturnType<typeof setInterval> | null = null;

  /** Horodatage du dernier cycle terminé, toutes sources confondues. */
  get lastSyncAt(): string | null {
    const stamps = (this.overview?.freshness ?? [])
      .map((f) => f.finished_at)
      .filter((value): value is string => value !== null);
    return stamps.length === 0 ? null : stamps.reduce((a, b) => (a > b ? a : b));
  }

  /** Âge des données en millisecondes, `null` si aucun cycle n'a jamais abouti. */
  get dataAgeMs(): number | null {
    const last = this.lastSyncAt;
    if (last === null) return null;
    const parsed = Date.parse(last);
    return Number.isNaN(parsed) ? null : Date.now() - parsed;
  }

  async refreshAuth() {
    this.auth = await api.authStatus();
  }

  /**
   * Démarrage : si un token est déjà enregistré, on charge la base, on
   * synchronise si les données sont périmées, puis on arme le réveil
   * périodique qui entretiendra les snapshots quotidiens CurseForge.
   */
  async boot() {
    await this.refreshAuth();
    if (!this.auth?.connected) return;
    await this.load();
    await this.autoSync();
    this.startAutoSync();
  }

  /** Arme le réveil de synchronisation. Sans effet s'il tourne déjà. */
  startAutoSync() {
    if (this.timer !== null) return;
    this.timer = setInterval(() => void this.autoSync(), AUTO_SYNC_TICK_MS);
  }

  /**
   * Synchronise si — et seulement si — les données le méritent : base vide,
   * échéancier de reversement jamais relevé, ou dernier cycle trop ancien.
   */
  async autoSync() {
    if (this.syncing || !this.auth?.connected) return;
    const age = this.dataAgeMs;
    const stale =
      this.overview === null ||
      this.overview.per_project.length === 0 ||
      this.overview.payout.available === "" ||
      age === null ||
      age > STALE_AFTER_MS;
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
      this.startAutoSync();
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

  /** Noms des plateformes retenues, tels qu'attendus par le backend. */
  get visiblePlatforms(): string[] {
    return Object.entries(this.platforms)
      .filter(([, on]) => on)
      .map(([name]) => name);
  }

  /**
   * Masque ou réaffiche une plateforme. La dernière visible ne peut pas être
   * masquée : un écran vide n'apprendrait rien.
   */
  async togglePlatform(name: "modrinth" | "curseforge") {
    if (this.platforms[name] && this.visiblePlatforms.length === 1) return;
    this.platforms = { ...this.platforms, [name]: !this.platforms[name] };
    await this.load();
  }

  async load() {
    this.loading = true;
    this.error = null;
    try {
      this.overview = await api.overview(
        this.rangeDays,
        this.rangeFrom,
        this.rangeTo,
        this.visiblePlatforms,
      );
    } catch (e) {
      this.error = message(e);
    } finally {
      this.loading = false;
    }
  }

  /** Recharge la vue courante, page de vision comme détail de mod. */
  private async reload() {
    await this.load();
    if (this.project) await this.openProject(this.project.summary);
  }

  /** Fenêtre glissante de `days` jours se terminant aujourd'hui. */
  async setRange(days: number) {
    this.rangeDays = days;
    this.rangeFrom = null;
    this.rangeTo = null;
    await this.reload();
  }

  /** Mois calendaire complet, `month` au format `YYYY-MM`. */
  async setMonth(month: string) {
    this.rangeFrom = `${month}-01`;
    this.rangeTo = lastDayOfMonth(month);
    await this.reload();
  }

  /** Plage libre, bornes incluses. */
  async setCustomRange(from: string, to: string) {
    if (!from || !to) return;
    this.rangeFrom = from;
    this.rangeTo = to;
    await this.reload();
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
        this.rangeFrom,
        this.rangeTo,
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
