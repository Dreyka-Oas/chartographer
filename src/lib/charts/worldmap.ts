import { compactNumber, countryLabel } from "../format";
import type { CountryTotal } from "../types";
import { DARK, escapeHtml, tooltip, type Palette } from "./theme";

/**
 * Infobulle d'un pays. Le format par défaut d'ECharts nomme la série, qui n'en a
 * pas ici : il affichait « série () ». On dit ce qui compte, le pays en toutes
 * lettres et son compte, et on l'annonce même pour un pays sans relevé.
 */
export function countryTooltipHtml(name: string, value: number, total: number): string {
  // Le code pays vient du relevé, et `countryLabel` le rend tel quel quand il
  // ne le reconnaît pas : il finit donc dans le HTML sans être passé nulle part.
  const label = escapeHtml(countryLabel(name));
  if (!Number.isFinite(value) || value <= 0) {
    return `<b>${label}</b><br>aucun téléchargement relevé`;
  }
  const percent = total > 0 ? ((value / total) * 100).toFixed(1).replace(".", ",") : "";
  const share = percent ? ` · ${percent} %` : "";
  return `<b>${label}</b><br>${compactNumber(value)} téléchargements${share}`;
}

/**
 * ECharts resserre les longitudes d'un quart par défaut, ce qui étire la carte
 * en largeur. On garde les degrés tels quels : le dessin est alors moins large
 * et se rapproche de la forme du panneau, qu'il remplit d'autant mieux.
 */
export const ASPECT_SCALE = 1;

/**
 * Proportions du dessin, l'Antarctique retirée, relevées sur le rendu.
 *
 * Le calcul théorique — 360° de longitude sur 143° de latitude — ne tombe pas
 * juste : ECharts ajuste la géométrie dans son cadre avec ses propres marges.
 * C'est donc une mesure, à reprendre si le fond de carte change.
 */
export const MAP_ASPECT = 2.12;

/**
 * Grossissement qui comble les bandes vides d'un panneau plus large que la
 * carte.
 *
 * Ajustée pour tenir entière, la carte laisse deux marges à gauche et à droite
 * dès que le panneau s'allonge. L'agrandir les réduit, mais rogne d'autant en
 * haut et en bas : le plafond est ce qu'on peut perdre en latitude sans
 * entamer la Nouvelle-Zélande ni le Groenland, soit un huitième environ.
 */
export function fillZoom(width: number, height: number, max = 1.15): number {
  if (!(width > 0) || !(height > 0)) return 1;
  return Math.min(max, Math.max(1, width / height / MAP_ASPECT));
}

/** Le code `??` agrège `XX` et la chaîne vide côté Rust : il n'a pas de géométrie. */
export function worldMapOption(countries: CountryTotal[], p: Palette = DARK, zoom = 1) {
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
        /*
         * Le centre est le milieu des terres restantes, l'Antarctique retirée
         * (voir `WorldMap.svelte`) : c'est autour de lui que le panneau rogne
         * quand il est plus large que la carte, et le grossissement se règle sur
         * sa forme (voir `fillZoom`) plutôt que sur une valeur figée, puisqu'il
         * change de la vue d'ensemble à la vue détaillée.
         */
        center: [10, 15],
        zoom,
        aspectScale: ASPECT_SCALE,
        itemStyle: { areaColor: p.empty, borderColor: p.grid },
        emphasis: { label: { show: false }, itemStyle: { areaColor: p.accent } },
        nameProperty: "iso_a2",
        data: mapped.map((c) => ({ name: c.country, value: c.downloads })),
      },
    ],
  };
}
