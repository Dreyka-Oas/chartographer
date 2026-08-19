import { compactNumber, formatDay, formatDayLong, formatMonth } from "../format";

export interface Palette {
  modrinth: string;
  curseforge: string;
  accent: string;
  text: string;
  textDim: string;
  grid: string;
  surface: string;
  /** Fond des pays sans donnée sur la carte, et bas de gamme des heatmaps. */
  empty: string;
}

export const DARK: Palette = {
  modrinth: "#00af5c",
  curseforge: "#f16436",
  accent: "#5ac8a8",
  text: "#e6ebf0",
  textDim: "#8b97a5",
  grid: "#262d36",
  surface: "#14181d",
  empty: "#161b21",
};

export const LIGHT: Palette = {
  modrinth: "#00874a",
  curseforge: "#d4501f",
  accent: "#1f9c7c",
  text: "#16202b",
  textDim: "#5c6b7a",
  grid: "#d8dee6",
  surface: "#ffffff",
  empty: "#e8ecf1",
};

export function palette(dark: boolean): Palette {
  return dark ? DARK : LIGHT;
}

export const BASE_GRID = { left: 48, right: 16, top: 24, bottom: 56, containLabel: true };

/** Une entrée du tooltip d'axe, telle qu'ECharts la transmet. */
export interface AxisParam {
  axisValue?: string;
  marker?: string;
  seriesName?: string;
  value?: number | string;
  /**
   * L'item de données. Les graphiques qui empilent eux-mêmes leurs courbes y
   * joignent `own` : la valeur propre de la série, quand `value` porte le
   * cumul dessiné. Sans cela le tooltip annoncerait la hauteur du tracé au
   * lieu du chiffre du mod.
   */
  data?: number | { value?: number; own?: number };
}

/** Ce qu'il faut annoncer pour une entrée : sa valeur propre, pas son cumul. */
function ownValue(entry: AxisParam): number {
  const item = entry.data;
  if (item && typeof item === "object" && typeof item.own === "number") return item.own;
  return Number(entry.value ?? 0);
}

/** Réglages du tooltip d'un axe de jours. */
export interface DayTooltipOptions {
  format?: (value: number) => string;
  /** Trie les lignes de la plus grosse à la plus petite. */
  sorted?: boolean;
  /**
   * Logo à poser devant le nom d'une série, quand elle en a un. Le tooltip
   * d'ECharts est du HTML : l'adresse y est insérée telle quelle, d'où le
   * filtre de `iconTag`.
   */
  icon?: (seriesName: string) => string | null | undefined;
}

/**
 * Vignette d'une série, ou un blanc de même largeur pour que les noms restent
 * alignés quand une seule série sur dix n'a pas de logo.
 *
 * Seules les adresses `http(s)` sans guillemet ni chevron sont posées : elles
 * viennent des plateformes, mais elles finissent dans un attribut HTML monté à
 * la main, et rien ne garantit leur forme.
 */
/**
 * Texte rendu inoffensif dans du HTML monté à la main.
 *
 * Les noms de séries sont des titres de mods, relevés sur les deux
 * plateformes. Ils finissent dans le corps d'un tooltip ECharts, qui rend du
 * HTML : un titre contenant `<img onerror=…>` s'exécuterait. Rien de tel n'a
 * été vu, mais le texte vient du réseau, et c'est la seule raison qui compte.
 */
export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function iconTag(url: string | null | undefined): string {
  const size = "width:14px;height:14px;vertical-align:-3px;margin-right:4px";
  if (typeof url !== "string" || !/^https?:\/\/[^"'<>\s]+$/.test(url)) {
    return `<span style="display:inline-block;${size}"></span>`;
  }
  return `<img src="${url}" alt="" style="${size};border-radius:3px;object-fit:cover">`;
}

/**
 * Axe de jours lisible : l'étiquette porte le jour et le mois abrégé au lieu
 * de la date ISO brute, et ECharts efface celles qui se chevaucheraient.
 */
export function dayAxis(days: string[], p: Palette) {
  return {
    type: "category",
    data: days,
    ...axisStyle(p),
    axisLabel: {
      color: p.textDim,
      hideOverlap: true,
      formatter: (value: string) => formatDay(value),
    },
  };
}

/** Axe de mois : `août 2026` plutôt que `2026-08`. */
export function monthAxis(months: string[], p: Palette) {
  return {
    type: "category",
    data: months,
    ...axisStyle(p),
    axisLabel: {
      color: p.textDim,
      hideOverlap: true,
      rotate: 45,
      formatter: (value: string) => formatMonth(value),
    },
  };
}

/** Corps du tooltip d'un axe de jours, titre en date complète. */
export function dayTooltipHtml(params: AxisParam[], options: DayTooltipOptions = {}): string {
  const { format = compactNumber, sorted = false, icon } = options;
  const head = formatDayLong(String(params[0]?.axisValue ?? ""));
  let sum = 0;
  const entries = sorted ? [...params].sort((a, b) => ownValue(b) - ownValue(a)) : params;
  const rows = entries.map((entry) => {
    const value = ownValue(entry);
    if (Number.isFinite(value)) sum += value;
    const amount = Number.isFinite(value) ? format(value) : "—";
    const name = String(entry.seriesName ?? "");
    const logo = icon ? iconTag(icon(name)) : "";
    // `marker` est un fragment forgé par ECharts, pas une valeur relevée : il
    // reste tel quel. Le nom du mod, lui, vient des plateformes.
    return `${entry.marker ?? ""} ${logo}${escapeHtml(name)} <b>${amount}</b>`;
  });
  // Plusieurs séries : la question qui vient d'abord est « combien ce jour-là,
  // en tout ». On la met au pied, séparée du détail par un filet.
  if (params.length > 1) {
    rows.push(
      `<span style="opacity:.65">──────────</span><br>Total <b>${format(sum)}</b>`,
    );
  }
  return [`<b>${head}</b>`, ...rows].join("<br>");
}

/** Tooltip d'axe temporel, daté au jour près. */
export function dayTooltip(p: Palette, options: DayTooltipOptions = {}) {
  return {
    trigger: "axis",
    confine: true,
    ...tooltip(p),
    formatter: (params: AxisParam[]) => dayTooltipHtml(params, options),
  };
}

export function axisStyle(p: Palette) {
  return {
    axisLine: { lineStyle: { color: p.grid } },
    axisLabel: { color: p.textDim },
    splitLine: { lineStyle: { color: p.grid, opacity: 0.4 } },
  };
}

export function tooltip(p: Palette) {
  return {
    backgroundColor: p.surface,
    borderColor: p.grid,
    textStyle: { color: p.text },
  };
}
