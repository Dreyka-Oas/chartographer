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
