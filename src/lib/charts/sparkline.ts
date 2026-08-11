export interface SparkPath {
  /** Tracé de la courbe. */
  line: string;
  /** Même tracé refermé sur la ligne de base, pour l'aplat. */
  area: string;
}

/**
 * Sparkline en pur SVG, sans moteur de graphique.
 *
 * Une instance de graphique par ligne de tableau devient intenable dès
 * quelques dizaines de projets : chacune alloue un canvas et un observateur de
 * taille. Un chemin SVG coûte quelques nœuds et se redimensionne tout seul.
 *
 * Les coordonnées sont exprimées dans une boîte `width × height` que le SVG
 * étire ensuite à la largeur disponible.
 */
export function sparklinePath(values: number[], width = 100, height = 30): SparkPath {
  if (values.length < 2) return { line: "", area: "" };

  const max = values.reduce((a, b) => (b > a ? b : a), 0);
  const scale = max > 0 ? height / max : 0;
  const step = width / (values.length - 1);
  const round = (value: number) => Math.round(value * 100) / 100;

  const points = values.map((value, index) => `${round(index * step)},${round(height - value * scale)}`);
  const line = `M${points.join("L")}`;
  const area = `${line}L${round(width)},${height}L0,${height}Z`;
  return { line, area };
}
