import { formatDayLong } from "../format";
import type { FollowerDay } from "../types";
import { axisStyle, BASE_GRID, DARK, dayAxis, type Palette, tooltip } from "./theme";

/** Écart d'un jour sur l'autre, signé, pour une série de comptes. */
function deltas(values: number[]): (number | null)[] {
  // Le premier jour n'a pas de veille : son écart est inconnu, pas nul.
  return values.map((value, index) => (index === 0 ? null : value - values[index - 1]));
}

const signed = (value: number) => (value > 0 ? `+${value}` : String(value));

/**
 * Évolution du nombre d'abonnés, plateforme par plateforme.
 *
 * Deux lectures dans un seul dessin : les aires empilées portent l'effectif —
 * ce que les plateformes annoncent —, les barres portent ce que l'application
 * a calculé elle-même, l'écart d'un jour sur l'autre. Sans elles, un compte qui
 * monte de deux sur cent cinquante ne se verrait pas ; sans les aires, on
 * perdrait de vue combien ils sont.
 *
 * L'axe des effectifs ne part pas de zéro : un compte d'abonnés bouge de
 * quelques unités sur des centaines, ramené à zéro le trait serait plat. Celui
 * des écarts, lui, est centré sur zéro — c'est la ligne qui sépare une arrivée
 * d'un départ.
 */
export function followersOption(days: FollowerDay[], p: Palette = DARK) {
  const axis = axisStyle(p);
  const totals = days.map((d) => d.modrinth + d.curseforge);
  const low = totals.length > 0 ? Math.min(...totals) : 0;
  const high = totals.length > 0 ? Math.max(...totals) : 0;
  // Une marge d'un dixième de l'écart, et jamais moins d'une unité : une courbe
  // parfaitement plate doit garder de l'air au-dessus et en dessous.
  const room = Math.max(1, Math.round((high - low) * 0.1));

  const modrinthGain = deltas(days.map((d) => d.modrinth));
  const curseforgeGain = deltas(days.map((d) => d.curseforge));
  // Les barres partagent une échelle symétrique : une perte de trois doit se
  // lire aussi bas qu'un gain de trois se lit haut.
  const swing = Math.max(
    1,
    ...modrinthGain.concat(curseforgeGain).map((value) => Math.abs(value ?? 0)),
  );

  return {
    grid: BASE_GRID,
    tooltip: {
      trigger: "axis",
      confine: true,
      ...tooltip(p),
      formatter: (params: { axisValue?: string; dataIndex?: number }[]) => {
        const index = params[0]?.dataIndex ?? 0;
        const day = days[index];
        if (!day) return "";
        const line = (name: string, count: number, gain: number | null, color: string) => {
          const move = gain === null || gain === 0 ? "" : ` <b>${signed(gain)}</b>`;
          const dot = `<span style="display:inline-block;width:9px;height:9px;border-radius:2px;background:${color}"></span>`;
          return `${dot} ${name} ${count}${move}`;
        };
        return [
          `<b>${formatDayLong(day.day)}</b>`,
          line("Modrinth", day.modrinth, modrinthGain[index], p.modrinth),
          line("CurseForge", day.curseforge, curseforgeGain[index], p.curseforge),
          `<span style="opacity:.65">──────────</span><br>Total <b>${day.modrinth + day.curseforge}</b>`,
        ].join("<br>");
      },
    },
    legend: {
      data: ["Modrinth", "CurseForge", "Écart Modrinth", "Écart CurseForge"],
      textStyle: { color: p.textDim },
      top: 0,
    },
    xAxis: dayAxis(
      days.map((d) => d.day),
      p,
    ),
    yAxis: [
      {
        type: "value",
        name: "abonnés",
        nameTextStyle: { color: p.textDim },
        min: Math.max(0, low - room),
        max: high + room,
        ...axis,
      },
      {
        type: "value",
        name: "écart",
        nameTextStyle: { color: p.textDim },
        min: -swing,
        max: swing,
        ...axis,
        // Une seule grille suffit : deux jeux de lignes se croiseraient sans
        // rien apporter.
        splitLine: { show: false },
      },
    ],
    series: [
      {
        name: "Modrinth",
        type: "line",
        stack: "followers",
        // En marches et non en courbe lissée : un abonné arrive d'un coup, il
        // ne se répartit pas sur la journée. Une pente adoucie inventerait une
        // progression que le relevé quotidien ne peut pas connaître.
        step: "end",
        showSymbol: days.length < 40,
        symbolSize: 5,
        areaStyle: { opacity: 0.22 },
        itemStyle: { color: p.modrinth },
        data: days.map((d) => d.modrinth),
      },
      {
        name: "CurseForge",
        type: "line",
        stack: "followers",
        step: "end",
        showSymbol: days.length < 40,
        symbolSize: 5,
        areaStyle: { opacity: 0.22 },
        itemStyle: { color: p.curseforge },
        data: days.map((d) => d.curseforge),
      },
      {
        name: "Écart Modrinth",
        type: "bar",
        yAxisIndex: 1,
        barMaxWidth: 14,
        itemStyle: { color: p.modrinth, opacity: 0.85 },
        data: modrinthGain,
      },
      {
        name: "Écart CurseForge",
        type: "bar",
        yAxisIndex: 1,
        barMaxWidth: 14,
        itemStyle: { color: p.curseforge, opacity: 0.85 },
        data: curseforgeGain,
      },
    ],
  };
}
