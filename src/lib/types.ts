export interface Kpis {
  downloads_total: number;
  downloads_modrinth: number;
  downloads_curseforge: number;
  downloads_30d: number;
  downloads_prev_30d: number;
  revenue_total: string;
  revenue_pending: string;
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

export interface Overview {
  kpis: Kpis;
  timeline: TimelinePoint[];
  per_project: ProjectSummary[];
  countries: CountryTotal[];
  loaders: LoaderCell[];
  revenue: RevenuePoint[];
  events: EventRow[];
  freshness: Freshness[];
  curseforge_history_days: number;
}

export interface AuthStatus {
  connected: boolean;
  username: string | null;
  connected_since: string | null;
  oauth_app_configured: boolean;
}

export interface Settings {
  curseforge_username: string | null;
  range_days: number;
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
