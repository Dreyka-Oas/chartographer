export interface Kpis {
  downloads_total: number;
  downloads_modrinth: number;
  downloads_curseforge: number;
  downloads_30d: number;
  downloads_30d_modrinth: number;
  downloads_30d_curseforge: number;
  downloads_prev_30d: number;
  /** Les deux plateformes réunies : reversement Modrinth et points CurseForge. */
  revenue_total: string;
  /** Part Modrinth : retiré, retirable et en maturation. */
  revenue_modrinth: string;
  /** Part CurseForge : le solde de points converti au tarif publié. */
  revenue_curseforge: string;
  revenue_available: string;
  /** Ce qui, dans la somme retirable, vient de chaque plateforme. */
  revenue_available_modrinth: string;
  revenue_available_curseforge: string;
  revenue_pending: string;
  /** Revenus relevés jour par jour sur la fenêtre affichée. */
  revenue_window: string;
  /** Téléchargements de la période choisie, et de celle de même durée d'avant. */
  range_downloads: number;
  range_downloads_modrinth: number;
  range_downloads_curseforge: number;
  range_downloads_prev: number;
  /** Revenus de la période. Ceux de CurseForge viennent de l'écart de solde. */
  range_revenue: string;
  range_revenue_modrinth: string;
  range_revenue_curseforge: string;
  followers: number;
  /** Abonnés par plateforme : ceux de CurseForge viennent du tableau de bord. */
  followers_modrinth: number;
  followers_curseforge: number;
  projects_active: number;
  projects_modrinth: number;
  projects_curseforge: number;
}

export interface TimelinePoint {
  day: string;
  modrinth: number;
  curseforge: number;
}

export interface ProjectSummary {
  key: string;
  title: string;
  icon_url: string | null;
  modrinth_id: number | null;
  curseforge_id: number | null;
  /** Identifiants connus des plateformes, seuls utilisables pour leur parler. */
  modrinth_ext_id: string | null;
  curseforge_ext_id: number | null;
  modrinth_downloads: number;
  curseforge_downloads: number;
  followers: number;
  link_confidence: number | null;
  spark: number[];
}

export interface CountryTotal {
  country: string;
  downloads: number;
}

export interface LoaderCell {
  game_version: string;
  loader: string;
  downloads: number;
}

export interface RevenuePoint {
  day: string;
  amount: string;
}

export interface PayoutPoint {
  date: string;
  amount: string;
  /** Échéance postérieure à aujourd'hui : revenu à venir. */
  future: boolean;
}

export interface Payout {
  available: string;
  pending: string;
  withdrawn_lifetime: string;
  withdrawn_ytd: string;
  schedule: PayoutPoint[];
}

export interface RevenueByProject {
  key: string;
  title: string;
  amount: string;
}

export interface EventRow {
  occurred_at: string;
  kind: string;
  title: string;
  detail: string;
}

export interface Freshness {
  provider: string;
  status: string;
  finished_at: string | null;
  detail: string;
}

export interface VersionRow {
  version_number: string | null;
  game_versions: string[];
  loaders: string[];
  downloads: number;
  date_published: string | null;
}

export interface ProjectDetail {
  summary: ProjectSummary;
  days: string[];
  downloads: number[];
  views: number[];
  curseforge: number[];
  revenue: string[];
  countries: CountryTotal[];
  versions: VersionRow[];
}

export interface Overview {
  kpis: Kpis;
  /** Bornes de la fenêtre affichée, toutes deux incluses. */
  from: string;
  to: string;
  /** Mois `YYYY-MM` disponibles en base, pour le filtre par mois. */
  available_months: string[];
  /** Axe de jours dense : toutes les séries par projet y sont alignées. */
  days: string[];
  timeline: TimelinePoint[];
  per_project: ProjectSummary[];
  countries: CountryTotal[];
  loaders: LoaderCell[];
  revenue: RevenuePoint[];
  revenue_by_project: RevenueByProject[];
  payout: Payout;
  events: EventRow[];
  freshness: Freshness[];
  curseforge_history_days: number;
  curseforge_revenue: CfRevenue;
  currency: CurrencyView;
}

/** Un mois de revenus relevé sur le tableau de bord CurseForge. */
export interface CfRevenueMonth {
  /** Mois `YYYY-MM`. */
  month: string;
  amount_usd: string;
}

/** Tout ce que CurseForge dit de l'argent. */
export interface CfRevenue {
  points: number;
  points_usd: string;
  /** Estimation du mois écoulé, en dollars, telle qu'affichée par le site. */
  last_month: string | null;
  /** Cumul de l'année en cours, en dollars. */
  year_to_date: string | null;
  monthly: CfRevenueMonth[];
}

export interface AuthStatus {
  connected: boolean;
  username: string | null;
  connected_since: string | null;
  /** Auteur CurseForge retenu, réglé à la main ou détecté au dernier cycle. */
  curseforge_username: string | null;
  modrinth_projects: number;
  curseforge_projects: number;
}

export interface Settings {
  curseforge_username: string | null;
  range_days: number;
  /** Devise d'affichage, code ISO à trois lettres. */
  currency: string;
  /** Minutes entre deux relevés automatiques, jamais moins de dix. */
  auto_sync_minutes: number;
  /** Vrai quand le jeton d'envoi CurseForge a été relevé. Le jeton lui-même
   * ne quitte jamais l'application. */
  curseforge_token_ready: boolean;
}

/** Devise d'affichage et taux appliqué aux montants, tous reçus en dollars. */
export interface CurrencyView {
  code: string;
  rate: number;
  /** Jour du taux, vide tant qu'aucun n'a été relevé. */
  day: string;
}

/** Une entrée du catalogue des versions de jeu CurseForge. */
export interface GameVersion {
  id: number;
  name: string;
  slug: string;
  type_id: number;
}

/** Une version prête à partir, décrite une fois pour les deux plateformes. */
export interface PublishDraft {
  modrinth_project_id: string | null;
  curseforge_project_id: number | null;
  name: string;
  version_number: string;
  changelog: string;
  game_versions: string[];
  loaders: string[];
  release_type: string;
  manual_release: boolean;
}

export interface PublishOutcome {
  platform: string;
  ok: boolean;
  id: string | null;
  detail: string;
}

export interface PublishReport {
  outcomes: PublishOutcome[];
}

/** Un geste du tableau de bord CurseForge, appris en le regardant faire. */
export interface CfGesture {
  method: string;
  /** Adresse où les identifiants ont laissé place à des repères `{1}`, `{2}`. */
  pattern: string;
  body: string;
  status: number;
}

export interface PairingEntry {
  id: number;
  platform: string;
  title: string;
  /** Identifiant et titre du jumeau, si le projet est apparié. */
  linked_id: number | null;
  linked_to: string | null;
  manual: boolean;
  /** Déclaré sans équivalent sur l'autre plateforme. */
  solo: boolean;
}

/** Relevé manuel du solde de points CurseForge. */
export interface CfPointEntry {
  day: string;
  points: number;
  /** Contre-valeur au tarif annoncé par CurseForge : 0,05 $ le point. */
  value_usd: string;
}

/** Une réponse interceptée dans la fenêtre CurseForge, résumée. */
export interface CfCapture {
  url: string;
  days: number;
  from: string | null;
  to: string | null;
  total: number;
}

/** Ce que la fenêtre CurseForge donne à lire. */
export interface CfScrape {
  url: string;
  title: string;
  points: number | null;
  excerpt: string;
  captures: CfCapture[];
}

/** Un historique rattaché à un mod lors de la collecte. */
export interface CfImported {
  title: string;
  days: number;
  from: string;
  to: string;
}

/** Résultat d'une collecte automatique sur le tableau de bord CurseForge. */
/** Un abonné CurseForge, relevé sur la fiche publique du compte. */
export interface Follower {
  name: string;
  avatar_url: string | null;
  /** Ancienneté du compte, telle que le site l'écrit. */
  seniority: string | null;
  /** Jour du premier relevé où ce nom est apparu. */
  first_seen: string;
  last_seen: string;
  /** Jour où il a cessé de figurer, s'il est parti. */
  lost_on: string | null;
  rank: number;
  /**
   * Faux quand il était déjà là au tout premier relevé : son abonnement est
   * antérieur, sans qu'on sache de combien.
   */
  arrival_known: boolean;
}

/** Une mesure du jour, ses parts, et ce à quoi la comparer. */
export interface DayFigure {
  total: number;
  modrinth: number;
  curseforge: number;
  previous: number;
  average_7: number;
  average_28: number;
}

export interface DayMoney {
  total: string;
  modrinth: string;
  curseforge: string;
  previous: string;
  average_7: string;
  average_28: string;
}

export interface DayProject {
  key: string;
  title: string;
  icon_url: string | null;
  modrinth: number;
  curseforge: number;
  total: number;
  previous: number;
}

/** Bilan d'une seule journée, avec de quoi la juger. */
export interface DayReport {
  day: string;
  /** Vrai quand la journée n'est pas finie : ses chiffres monteront encore. */
  partial: boolean;
  downloads: DayFigure;
  revenue: DayMoney;
  /** Rang parmi les journées relevées, 1 étant la meilleure. */
  rank: number | null;
  ranked_days: number;
  best_day: string | null;
  best_downloads: number;
  followers_delta: number | null;
  projects: DayProject[];
  events: EventRow[];
}

/** Une journée dans le classement, avec le rang que les réglages ont produit. */
export interface DayRankRow {
  day: string;
  modrinth: number;
  curseforge: number;
  total: number;
  /** Revenus du jour, en dollars, tels que la base les connaît. */
  revenue: string;
  /** Rang de la journée selon les réglages demandés, 1 étant le meilleur. */
  rank: number | null;
  /** Journées réellement comparées pour établir ce rang. */
  compared_days: number;
}

/** Ce sur quoi les journées se classent. */
export type RankBy = "downloads" | "revenue";

/**
 * À quoi une journée se compare pour obtenir son rang.
 *
 * Deux familles. `all` et `period` sont absolues : elles comparent un groupe
 * de journées à lui-même, sans égard à leur ordre — `all` sur tout
 * l'historique, `period` sur la seule période affichée. `sliding` et
 * `retrospective` sont rétrospectives : une journée n'y est jugée que sur
 * celles qui la précèdent, si bien qu'un rang une fois acquis ne bouge plus
 * jamais — `sliding` sur une fenêtre de jours, `retrospective` sans borne
 * basse.
 */
export type RankScope = "sliding" | "retrospective" | "all" | "period";

export interface DayRankings {
  /** Les journées relevées, de la plus ancienne à la plus récente. */
  rows: DayRankRow[];
  first_modrinth_day: string | null;
  first_curseforge_day: string | null;
}

/** Un jour de la courbe des abonnés, plateforme par plateforme. */
export interface FollowerDay {
  day: string;
  modrinth: number;
  curseforge: number;
}

export interface FollowersReport {
  followers: Follower[];
  /** Évolution du compte, un point par jour relevé. */
  history: FollowerDay[];
  /** Nombre annoncé par la page, quand elle l'affiche. */
  announced: number | null;
  arrived: number;
  lost: number;
  since: string | null;
  detail: string;
}

/** Réponse de la page CurseForge quand on lui demande si le compte est ouvert. */
export interface CfSession {
  connected: boolean;
  url: string;
}

export interface CfCollect {
  needs_login: boolean;
  visited: string[];
  imported: CfImported[];
  points: number | null;
  detail: string;
  /** Rempli par le front quand la collecte n'a pas pu aboutir. Une collecte
   * ratée ne dit rien des séries : c'est un échec, pas un résultat vide. */
  failed?: boolean;
}

/** Ce que l'application reconnaît dans un contenu rapporté de CurseForge. */
export interface CfAnalysis {
  /** Solde de points repéré dans du texte copié. */
  points: number | null;
  /** Jours de la série datée trouvée, le cas échéant. */
  days: number;
  from: string | null;
  to: string | null;
  total: number;
  excerpt: string;
}

export interface SyncReport {
  provider: string;
  status: string;
  detail: string;
}

/** Jalon posé par le cycle de synchronisation, à l'ouverture puis à la clôture
 * de chacune de ses étapes. Le compte rendu est nul tant qu'elle est en cours. */
export interface SyncStep {
  provider: string;
  report: SyncReport | null;
}

export interface AppErrorPayload {
  kind: string;
  message: string;
}
