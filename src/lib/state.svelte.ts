import { api } from "./api";
import { lastDayOfMonth, setCurrency } from "./format";
import type {
  AppErrorPayload,
  AuthStatus,
  CfCollect,
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

/**
 * Cadence par défaut, en minutes, tant que les réglages n'ont pas répondu.
 * Le plancher réel est imposé côté Rust (`clamp_auto_sync`).
 */
const DEFAULT_AUTO_SYNC_MINUTES = 10;

/**
 * Dispersion appliquée à chaque attente, en fraction de la cadence.
 *
 * Un relevé qui tombe à la seconde près, indéfiniment, est la signature d'un
 * automate — et CurseForge ne se lit qu'à travers une session de navigateur,
 * donc sous les mêmes yeux qu'un visiteur. L'attente varie de plus ou moins un
 * quart, ce qui suffit à casser la régularité sans changer la fréquence
 * moyenne demandée.
 */
const JITTER = 0.25;

class Dashboard {
  detail = $state<DetailView | null>(null);
  project = $state<ProjectDetail | null>(null);
  projectLoading = $state(false);
  auth = $state<AuthStatus | null>(null);
  overview = $state<Overview | null>(null);
  rangeDays = $state(30);
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
  /** Cadence des relevés automatiques, telle que les réglages l'ont fixée. */
  autoSyncMinutes = $state(DEFAULT_AUTO_SYNC_MINUTES);
  private timer: ReturnType<typeof setTimeout> | null = null;

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
    // La cadence vient des réglages : la lire avant d'armer le réveil évite un
    // premier cycle à la mauvaise fréquence.
    try {
      this.autoSyncMinutes = (await api.getSettings()).auto_sync_minutes;
    } catch {
      this.autoSyncMinutes = DEFAULT_AUTO_SYNC_MINUTES;
    }
    await this.load();
    const synced = await this.autoSync();
    // La collecte CurseForge n'attend pas la péremption générale : elle est la
    // seule source de son historique et ne coûte qu'une fenêtre cachée. Elle
    // est déjà faite si une synchronisation vient d'avoir lieu.
    if (!synced) {
      await this.collectCurseforge();
      await this.load();
    }
    this.startAutoSync();
  }

  /** Cadence en millisecondes, plancher de dix minutes comme côté Rust. */
  get autoSyncMs(): number {
    return Math.max(10, this.autoSyncMinutes) * 60_000;
  }

  /** Attente jusqu'au prochain réveil, dispersée autour de la cadence. */
  private nextDelayMs(): number {
    const spread = this.autoSyncMs * JITTER;
    return this.autoSyncMs + (Math.random() * 2 - 1) * spread;
  }

  /**
   * Arme le réveil de synchronisation. Sans effet s'il tourne déjà.
   *
   * Chaque attente est retirée au sort plutôt que fixée une fois pour toutes :
   * un `setInterval` rendrait les relevés parfaitement périodiques.
   */
  startAutoSync() {
    if (this.timer !== null) return;
    const arm = () => {
      this.timer = setTimeout(() => {
        this.timer = null;
        void this.autoSync().finally(arm);
      }, this.nextDelayMs());
    };
    arm();
  }

  /** Coupe le réveil, puis le réarme sur la cadence courante. */
  restartAutoSync(minutes: number) {
    this.autoSyncMinutes = minutes;
    if (this.timer !== null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    this.startAutoSync();
  }

  /**
   * Synchronise si — et seulement si — les données le méritent : base vide,
   * échéancier de reversement jamais relevé, ou dernier cycle trop ancien.
   * Rend vrai quand un cycle a réellement eu lieu.
   */
  async autoSync(): Promise<boolean> {
    if (this.syncing || !this.auth?.connected) return false;
    const age = this.dataAgeMs;
    const stale =
      this.overview === null ||
      this.overview.per_project.length === 0 ||
      this.overview.payout.available === "" ||
      age === null ||
      age > this.autoSyncMs;
    if (stale) await this.sync();
    return stale;
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
      // Les montants arrivent en dollars : la mise en forme applique la devise
      // choisie et son taux, une fois pour toute la page.
      setCurrency(this.overview.currency.code, this.overview.currency.rate);
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
      // Le tableau de bord CurseForge se relève dans la foulée : sa fenêtre
      // reste invisible tant que la session tient.
      await this.collectCurseforge();
      await this.load();
    } catch (e) {
      this.error = message(e);
    } finally {
      this.syncing = false;
    }
  }

  /**
   * Relève le taux de change de la devise choisie, puis redessine la page avec.
   * Sans taux, un montant en euros serait un montant en dollars mal habillé.
   */
  async refreshCurrency() {
    try {
      await api.refreshExchangeRate();
      await this.load();
    } catch (e) {
      this.error = message(e);
    }
  }

  /** Dernier compte rendu de la collecte CurseForge, affiché dans les réglages. */
  curseforge = $state<CfCollect | null>(null);

  async collectCurseforge() {
    try {
      this.curseforge = await api.collectCurseforge();
    } catch (e) {
      // Une collecte ratée ne doit pas faire échouer la synchronisation, mais
      // elle ne doit pas non plus se lire comme un relevé vide.
      this.curseforge = {
        needs_login: false,
        visited: [],
        imported: [],
        points: null,
        detail: message(e),
        failed: true,
      };
    }
  }
}

export const dashboard = new Dashboard();
