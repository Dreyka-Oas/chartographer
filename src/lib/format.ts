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

export function countryLabel(code: string): string {
  if (code === "??") return "Inconnu";
  try {
    return new Intl.DisplayNames(["fr"], { type: "region" }).of(code) ?? code;
  } catch {
    return code;
  }
}
