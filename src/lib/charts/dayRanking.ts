import { PODIUM } from "../components/rank";
import type { DayRankRow } from "../types";
import { axisStyle, BASE_GRID, DARK, dayAxis, dayTooltip, type Palette } from "./theme";

/** Zoom commun aux deux vues : un an de journées ne tient pas à l'écran. */
const ZOOM = [
  { type: "inside", start: 0, end: 100 },
  { type: "slider", height: 20, bottom: 8 },
];

/**
 * Les journées de la période, barre par barre.
 *
 * Le podium est marqué d'un liseré plutôt que d'une couleur pleine : la barre
 * dit déjà la plateforme, et la repeindre ferait perdre cette lecture-là pour
 * en gagner une autre.
 */
export function dailyBarsOption(rows: DayRankRow[], p: Palette = DARK) {
  const axis = axisStyle(p);
  const crown = (row: DayRankRow, top: boolean) => {
    const color = row.rank_period !== null ? PODIUM[row.rank_period - 1] : undefined;
    return top && color ? { itemStyle: { borderColor: color, borderWidth: 2 } } : {};
  };
  return {
    grid: BASE_GRID,
    tooltip: dayTooltip(p, { sorted: true }),
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: p.textDim }, top: 0 },
    xAxis: dayAxis(rows.map((r) => r.day), p),
    yAxis: { type: "value", ...axis },
    dataZoom: ZOOM.map((z) => ({ ...z, borderColor: p.grid, textStyle: { color: p.textDim } })),
    series: [
      {
        id: "day:modrinth",
        name: "Modrinth",
        type: "bar",
        stack: "jour",
        itemStyle: { color: p.modrinth },
        data: rows.map((r) => ({ value: r.modrinth, ...crown(r, r.curseforge === 0) })),
      },
      {
        id: "day:curseforge",
        name: "CurseForge",
        type: "bar",
        stack: "jour",
        itemStyle: { color: p.curseforge },
        data: rows.map((r) => ({ value: r.curseforge, ...crown(r, r.curseforge > 0) })),
      },
    ],
  };
}

/**
 * Le rang qu'avait chaque journée le jour où elle s'est produite.
 *
 * L'axe est retourné : un premier rang est un sommet, et le lire au fond du
 * graphique demanderait au lecteur de renverser mentalement toute la courbe.
 * Les journées sans relevé laissent un trou plutôt qu'un zéro, qui se lirait
 * comme un rang. Le tooltip garde le rang en entier brut : `compactNumber`
 * est pensé pour des téléchargements, pas pour un classement.
 */
export function rankCurveOption(rows: DayRankRow[], p: Palette = DARK) {
  const axis = axisStyle(p);
  return {
    grid: BASE_GRID,
    tooltip: dayTooltip(p, { format: (value: number) => String(value) }),
    xAxis: dayAxis(rows.map((r) => r.day), p),
    yAxis: { type: "value", inverse: true, min: 1, ...axis },
    dataZoom: ZOOM.map((z) => ({ ...z, borderColor: p.grid, textStyle: { color: p.textDim } })),
    series: [
      {
        id: "day:rank",
        name: "Rang du jour",
        type: "line",
        step: "middle",
        showSymbol: false,
        connectNulls: false,
        itemStyle: { color: p.accent },
        data: rows.map((r) => r.rank_at_the_time),
      },
    ],
  };
}
