const THIN_SPACE = " ";

export function compactNumber(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1).replace(".", ",")}${THIN_SPACE}M`;
  }
  if (abs >= 1_000) {
    return `${(value / 1_000).toFixed(1).replace(".", ",")}${THIN_SPACE}k`;
  }
  return String(value);
}

export function deltaPercent(current: number, previous: number): number | null {
  if (previous === 0) return null;
  return Math.round(((current - previous) / previous) * 100);
}

export function formatMoney(raw: string): string {
  const value = Number.parseFloat(raw);
  const safe = Number.isFinite(value) ? value : 0;
  return `${safe.toFixed(2).replace(".", ",")}${THIN_SPACE}$`;
}

const MONTHS = [
  "janv.",
  "févr.",
  "mars",
  "avril",
  "mai",
  "juin",
  "juil.",
  "août",
  "sept.",
  "oct.",
  "nov.",
  "déc.",
];

export function formatDay(iso: string): string {
  const [, month, day] = iso.split("-");
  const index = Number.parseInt(month ?? "1", 10) - 1;
  return `${Number.parseInt(day ?? "1", 10)} ${MONTHS[index] ?? ""}`;
}

/** Jour daté de son année : `11 août 2026`. */
export function formatDayLong(iso: string): string {
  const [year] = iso.split("-");
  return `${formatDay(iso)} ${year ?? ""}`.trim();
}

/** Mois `YYYY-MM` en toutes lettres : `août 2026`. */
export function formatMonth(month: string): string {
  const [year, index] = month.split("-");
  const name = MONTHS[Number.parseInt(index ?? "1", 10) - 1] ?? month;
  return `${name} ${year ?? ""}`.trim();
}

/**
 * Dernier jour d'un mois `YYYY-MM`. Le jour 0 du mois suivant est le dernier du
 * mois courant, ce qui gère les années bissextiles sans table de longueurs.
 */
export function lastDayOfMonth(month: string): string {
  const [year, index] = month.split("-");
  const date = new Date(
    Date.UTC(Number.parseInt(year ?? "1970", 10), Number.parseInt(index ?? "1", 10), 0),
  );
  return date.toISOString().slice(0, 10);
}

/** Libellé d'une fenêtre, bornes incluses : `1 août → 31 août 2026`. */
export function formatRange(from: string, to: string): string {
  return `${formatDay(from)} → ${formatDayLong(to)}`;
}

/** Ancienneté en clair : `à l'instant`, `il y a 3 h`, `il y a 2 j`. */
export function formatAge(ms: number | null): string {
  if (ms === null || !Number.isFinite(ms)) return "jamais";
  const minutes = Math.floor(ms / 60_000);
  if (minutes < 2) return "à l'instant";
  if (minutes < 60) return `il y a ${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `il y a ${hours} h`;
  return `il y a ${Math.floor(hours / 24)} j`;
}

export function countryLabel(code: string): string {
  if (code === "??") return "Inconnu";
  try {
    return new Intl.DisplayNames(["fr"], { type: "region" }).of(code) ?? code;
  } catch {
    return code;
  }
}
