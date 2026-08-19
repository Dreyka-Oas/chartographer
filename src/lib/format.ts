const THIN_SPACE = " ";

export function compactNumber(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1).replace(".", ",")}${THIN_SPACE}M`;
  }
  if (abs >= 1_000) {
    return `${(value / 1_000).toFixed(1).replace(".", ",")}${THIN_SPACE}k`;
  }
  // Sous le millier, la valeur est écrite telle quelle — mais un montant a des
  // décimales, et `String` les sépare d'un point. La virgule est la seule
  // marque décimale employée ailleurs sur la page.
  return String(value).replace(".", ",");
}

/**
 * Nombre gradué sur un axe : `1 000`, `50 000`, `2 620`.
 *
 * Les axes n'abrègent pas. `compactNumber` arrondit à la décimale, ce qui suffit
 * pour un total isolé mais écrase les graduations d'un axe resserré : une courbe
 * d'abonnés allant de 2 620 à 2 760 affichait « 2,7 k » sur trois graduations de
 * suite, trois traits distincts portant le même nombre. Les milliers sont
 * séparés d'une espace fine insécable, jamais d'une virgule — elle marque la
 * décimale partout ailleurs sur la page.
 */
export function axisNumber(value: number): string {
  if (!Number.isFinite(value)) return "";
  const [entier, decimales] = Math.abs(value).toFixed(Number.isInteger(value) ? 0 : 2).split(".");
  const groupe = entier.replace(/\B(?=(\d{3})+(?!\d))/g, " ");
  const signe = value < 0 ? "-" : "";
  // Les décimales ne paraissent que si elles disent quelque chose : un axe de
  // montants gradué au dixième d'euro n'a pas à écrire `1,50` en `1,5`.
  const reste = decimales ? `,${decimales.replace(/0+$/, "")}` : "";
  return `${signe}${groupe}${reste === "," ? "" : reste}`;
}

/**
 * Part en pourcentage, à la décimale : `33,3 %`.
 *
 * Le signe est collé par l'appelant, qui choisit son espace. Ce qui compte ici
 * est la virgule : `toFixed` rend un point, qui se lit comme un séparateur de
 * milliers dans une page où tout le reste est en français.
 */
export function formatPercent(part: number, whole: number, digits = 1): string {
  if (!(whole > 0) || !Number.isFinite(part)) return "0";
  return ((part / whole) * 100).toFixed(digits).replace(".", ",");
}

export function deltaPercent(current: number, previous: number): number | null {
  if (previous === 0) return null;
  return Math.round(((current - previous) / previous) * 100);
}

/** Symboles des devises proposées ; le code sert de repli pour les autres. */
const SYMBOLS: Record<string, string> = {
  USD: "$",
  EUR: "€",
  GBP: "£",
  CHF: "CHF",
  CAD: "$ CA",
  JPY: "¥",
};

/**
 * Devise d'affichage et taux du dollar vers celle-ci. Les deux plateformes
 * paient en dollars : tout montant reçu est converti au moment de l'écrire.
 */
let money = { code: "USD", rate: 1, symbol: "$" };

export function setCurrency(code: string, rate: number) {
  const upper = (code || "USD").toUpperCase();
  money = {
    code: upper,
    rate: Number.isFinite(rate) && rate > 0 ? rate : 1,
    symbol: SYMBOLS[upper] ?? upper,
  };
}

export function currencyCode(): string {
  return money.code;
}

export function formatMoney(raw: string): string {
  const value = Number.parseFloat(raw);
  const safe = Number.isFinite(value) ? value : 0;
  const converted = safe * money.rate;
  return `${converted.toFixed(2).replace(".", ",")}${THIN_SPACE}${money.symbol}`;
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

/**
 * Taille d'un fichier en clair : `4,2 Mo`.
 *
 * Les paliers sont ceux du système décimal, comme les affiche Windows pour un
 * téléchargement : mille octets par unité, pas mille vingt-quatre. Un chiffre
 * après la virgule suffit — la taille sert à jauger l'attente, pas à compter.
 */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  const units = ["o", "ko", "Mo", "Go"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = unit === 0 ? 0 : 1;
  return `${value.toFixed(digits).replace(".", ",")} ${units[unit]}`;
}

export function countryLabel(code: string): string {
  if (code === "??") return "Inconnu";
  try {
    return new Intl.DisplayNames(["fr"], { type: "region" }).of(code) ?? code;
  } catch {
    return code;
  }
}
