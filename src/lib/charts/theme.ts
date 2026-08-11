export const COLORS = {
  modrinth: "#00af5c",
  curseforge: "#f16436",
  accent: "#5ac8a8",
  text: "#e6ebf0",
  textDim: "#8b97a5",
  grid: "#262d36",
  surface: "#14181d",
};

export const BASE_GRID = { left: 48, right: 16, top: 24, bottom: 56, containLabel: true };

export const AXIS_STYLE = {
  axisLine: { lineStyle: { color: COLORS.grid } },
  axisLabel: { color: COLORS.textDim },
  splitLine: { lineStyle: { color: COLORS.grid, opacity: 0.4 } },
};

export const TOOLTIP = {
  backgroundColor: COLORS.surface,
  borderColor: COLORS.grid,
  textStyle: { color: COLORS.text },
};
