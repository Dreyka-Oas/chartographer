import { compactNumber, countryLabel } from "../format";
import type { CountryTotal } from "../types";
import { DARK, tooltip, type Palette } from "./theme";

/**
 * Infobulle d'un pays. Le format par défaut d'ECharts nomme la série, qui n'en a
 * pas ici : il affichait « série () ». On dit ce qui compte, le pays en toutes
 * lettres et son compte, et on l'annonce même pour un pays sans relevé.
 */
export function countryTooltipHtml(name: string, value: number, total: number): string {
  const label = countryLabel(name);
  if (!Number.isFinite(value) || value <= 0) {
    return `<b>${label}</b><br>aucun téléchargement relevé`;
  }
  const percent = total > 0 ? ((value / total) * 100).toFixed(1).replace(".", ",") : "";
  const share = percent ? ` · ${percent} %` : "";
  return `<b>${label}</b><br>${compactNumber(value)} téléchargements${share}`;
}

/** Le code `??` agrège `XX` et la chaîne vide côté Rust : il n'a pas de géométrie. */
export function worldMapOption(countries: CountryTotal[], p: Palette = DARK) {
  const mapped = countries.filter((c) => c.country !== "??");
  const max = mapped.reduce((acc, c) => Math.max(acc, c.downloads), 0);
  const total = mapped.reduce((acc, c) => acc + c.downloads, 0);

  return {
    tooltip: {
      trigger: "item",
      confine: true,
      ...tooltip(p),
      formatter: (params: { name?: string; value?: number }) =>
        countryTooltipHtml(String(params.name ?? ""), Number(params.value), total),
    },
    visualMap: {
      min: 0,
      max: Math.max(max, 1),
      left: 12,
      bottom: 12,
      calculable: true,
      textStyle: { color: p.textDim },
      inRange: { color: [p.empty, p.accent, p.curseforge] },
    },
    series: [
      {
        type: "map",
        map: "world",
        roam: true,
        itemStyle: { areaColor: p.empty, borderColor: p.grid },
        emphasis: { label: { show: false }, itemStyle: { areaColor: p.accent } },
        nameProperty: "iso_a2",
        data: mapped.map((c) => ({ name: c.country, value: c.downloads })),
      },
    ],
  };
}
