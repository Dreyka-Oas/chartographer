import { api } from "./api";
import { SYNC_KEYS } from "./boot";
import { boot } from "./boot.svelte";
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
  | "projects"
  | "followers"
  | "days";

/**
 * Cadence par défaut, en minutes, tant que les réglages n'ont pas répondu.
 * Le plancher réel est imposé côté Rust (`clamp_auto_sync`).
 */
const DEFAULT_AUTO_SYNC_MINUTES = 10;

/**
 * Dispersion appliquée à chaque attente, en fraction de la cadence.
 *
 * Un relevé qui tombe à la seconde près, indéfiniment, est la signature d'un
 * automate, et CurseForge ne se lit qu'à travers une session de navigateur,
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
  /**
   * Lecture de chaque indicateur de tête, carte par carte. Faux, la carte dit
   * l'état du compte, cumul depuis l'origine, solde retirable. Vrai, elle se
   * rapporte à la période choisie dans la barre de filtres.
   *
   * Le réglage est propre à chaque carte : comparer un cumul et une période
   * côte à côte est justement ce qu'on veut souvent faire. Les deux jeux de
   * chiffres arrivent ensemble, la bascule n'appelle donc rien.
   */
  kpiRanged = $state([false, false, false, false]);
  loading = $state(false);
  syncing = $state(false);
  connecting = $state(false);
  error = $state<string | null>(null);
  lastSync = $state<SyncReport[]>([]);
  selectedProject = $state<string | null>(null);
  /** Cadence des relevés automatiques, telle que les réglages l'ont fixée. */
  autoSyncMinutes = $state(DEFAULT_AUTO_SYNC_MINUTES);
  /**
   * Session CurseForge, telle que la page l'a répondu. `null` tant que la
   * question n'a pas été posée : l'application ne peut alors rien affirmer, ce
   * qui n'est pas la même chose qu'une session absente.
   */
  curseforgeSession = $state<boolean | null>(null);
  checkingCurseforge = $state(false);
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
   * Demande à la fenêtre CurseForge si le compte y est ouvert.
   *
   * La réponse vient de la page, jamais des réglages : un pseudo relevé un jour
   * y reste inscrit longtemps après l'expiration de la session.
   */
  async checkCurseforge(): Promise<boolean> {
    if (this.checkingCurseforge) return this.curseforgeSession === true;
    this.checkingCurseforge = true;
    try {
      this.curseforgeSession = (await api.curseforgeSession()).connected;
    } catch (e) {
      // Une fenêtre qui n'a pas pu s'ouvrir ne prouve pas une session absente,
      // mais elle ne la prouve pas non plus : on reste sur "pas connecté".
      this.curseforgeSession = false;
      this.error = message(e);
    } finally {
      this.checkingCurseforge = false;
    }
    await this.enterIfReady();
    return this.curseforgeSession === true;
  }

  /**
   * Démarrage : on établit l'état des deux comptes, et la suite s'enchaîne
   * d'elle-même dès qu'ils sont tous les deux reliés, au lancement comme
   * après une connexion faite depuis l'écran d'accueil.
   *
   * Rien n'est rendu tant que ce chemin n'a pas abouti : l'écran d'ouverture
   * couvre la page et rapporte chaque étape.
   */
  async start() {
    boot.reset();
    boot.open("comptes");
    // Les jalons du cycle Rust arrivent par événement. Hors de la fenêtre de
    // l'application, une page ouverte dans un navigateur, le temps d'un
    // essai, il n'y a rien à écouter : les comptes rendus finaux suffiront.
    void api.onSyncStep((step) => boot.fromSync(step)).catch(() => {});
    try {
      await this.refreshAuth();
    } catch (e) {
      // Sans réponse du socle, il n'y a ni page à ouvrir ni compte à relier :
      // l'écran de démarrage garde la main et propose de reprendre.
      this.error = message(e);
      // La note reste courte : le message entier a sa place sous la liste.
      boot.close("comptes", false, "sans réponse");
      return;
    }
    await this.checkCurseforge();
  }

  /** Vrai une fois la mise en route faite, pour ne pas la refaire. */
  private entered = false;

  /**
   * Charge la base, synchronise si les données sont périmées, puis arme le
   * réveil périodique qui entretiendra les snapshots quotidiens CurseForge.
   *
   * Ne fait rien tant que les deux comptes ne sont pas reliés : l'application
   * les demande tous les deux, et une collecte à moitié aveugle ne produirait
   * que des totaux trompeurs.
   */
  private async enterIfReady() {
    if (this.entered) return;
    if (!this.auth?.connected || this.curseforgeSession !== true) {
      // Un compte manque : l'écran de démarrage s'efface sur la page de
      // connexion, et reprendra la main dès que les deux répondront présents.
      boot.release();
      return;
    }
    this.entered = true;
    boot.reset();
    boot.close("comptes", true, this.auth.username ?? "");

    // La cadence vient des réglages : la lire avant d'armer le réveil évite un
    // premier cycle à la mauvaise fréquence.
    boot.open("reglages");
    try {
      this.autoSyncMinutes = (await api.getSettings()).auto_sync_minutes;
      boot.close("reglages", true, `relevé toutes les ${this.autoSyncMinutes} minutes`);
    } catch {
      this.autoSyncMinutes = DEFAULT_AUTO_SYNC_MINUTES;
      boot.close("reglages", false, "valeurs par défaut");
    }

    // Tout est relevé avant d'ouvrir, sans regarder l'âge des données : une
    // page qui paraît d'abord sur les chiffres d'hier, puis se corrige d'un
    // coup quelques secondes plus tard, ne se lit pas, on ignore, devant un
    // total bas, s'il est faible ou seulement pas encore arrivé.
    await this.sync();
    boot.release();
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

  /** Désarme le réveil. Sans effet s'il ne tourne pas. */
  stopAutoSync() {
    if (this.timer === null) return;
    clearTimeout(this.timer);
    this.timer = null;
  }

  /** Coupe le réveil, puis le réarme sur la cadence courante. */
  restartAutoSync(minutes: number) {
    this.autoSyncMinutes = minutes;
    this.stopAutoSync();
    this.startAutoSync();
  }

  /**
   * Synchronise si, et seulement si, les données le méritent : base vide,
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
   * Valide le token côté Rust avant de l'enregistrer, puis passe la main à la
   * mise en route, qui n'aura lieu que si CurseForge répond aussi présent.
   */
  async connect(token: string) {
    this.connecting = true;
    this.error = null;
    try {
      this.auth = await api.connect(token);
      await this.checkCurseforge();
    } catch (e) {
      this.error = message(e);
    } finally {
      this.connecting = false;
    }
  }

  async logout() {
    this.auth = await api.logout();
    this.overview = null;
    // La prochaine connexion refera la mise en route depuis le début : sans
    // cela, l'application reviendrait sur des chiffres qui ne sont plus lus.
    this.entered = false;
    boot.reset();
    boot.release();
    this.stopAutoSync();
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
      boot.adopt(this.lastSync);
      boot.settle(SYNC_KEYS, true);
      // Le tableau de bord CurseForge se relève dans la foulée : sa fenêtre
      // reste invisible tant que la session tient.
      boot.open("tableau");
      await this.collectCurseforge();
      boot.close("tableau", this.curseforge?.failed !== true, this.curseforge?.detail ?? "");
      boot.open("agregation");
      await this.load();
      boot.close("agregation", this.error === null, this.error ?? this.countedMods());
    } catch (e) {
      this.error = message(e);
      boot.settle([...SYNC_KEYS, "tableau", "agregation"], false, "interrompu");
    } finally {
      this.syncing = false;
    }
  }

  /** Ce que la page a fini par rassembler, dit en une ligne. */
  private countedMods(): string {
    const mods = this.overview?.per_project.length ?? 0;
    return `${mods} mod${mods > 1 ? "s" : ""} sur la période`;
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
