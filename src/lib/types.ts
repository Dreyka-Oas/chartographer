export interface Kpis {
  downloads_total: number;
  downloads_modrinth: number;
  downloads_curseforge: number;
  downloads_30d: number;
  downloads_prev_30d: number;
  /** Tout ce que le reversement a rapporté : retiré, retirable et en maturation. */
  revenue_total: string;
  revenue_available: string;
  revenue_pending: string;
  /** Revenus relevés jour par jour sur la fenêtre affichée. */
  revenue_window: string;
  followers: number;
  projects_active: number;
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

export interface AppErrorPayload {
  kind: string;
  message: string;
}
