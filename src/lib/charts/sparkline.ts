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

  const points = values.map((value, index) => ({
    x: round(index * step),
    y: round(height - value * scale),
  }));
  const line = smooth(points, height);
  const last = points[points.length - 1];
  const area = `${line}L${round(last.x)},${height}L${round(points[0].x)},${height}Z`;
  return { line, area };
}

interface Point {
  x: number;
  y: number;
}

/**
 * Relie les points par des Béziers plutôt que par des segments.
 *
 * Les mains de contrôle suivent Catmull-Rom : chacune s'aligne sur la pente
 * entre les deux voisins du point, ce qui donne une courbe qui passe par tous
 * les relevés au lieu de les approcher. La tension est retenue à un sixième et
 * les mains sont bornées à la boîte, sinon un pic isolé fait sortir la courbe
 * du cadre et l'aplat déborde sur la ligne du dessus.
 */
function smooth(points: Point[], height: number): string {
  const round = (value: number) => Math.round(value * 100) / 100;
  const clamp = (value: number) => Math.min(height, Math.max(0, value));
  let path = `M${points[0].x},${points[0].y}`;

  for (let i = 0; i < points.length - 1; i += 1) {
    const previous = points[i - 1] ?? points[i];
    const current = points[i];
    const next = points[i + 1];
    const after = points[i + 2] ?? next;

    const c1 = {
      x: round(current.x + (next.x - previous.x) / 6),
      y: round(clamp(current.y + (next.y - previous.y) / 6)),
    };
    const c2 = {
      x: round(next.x - (after.x - current.x) / 6),
      y: round(clamp(next.y - (after.y - current.y) / 6)),
    };
    path += `C${c1.x},${c1.y} ${c2.x},${c2.y} ${next.x},${next.y}`;
  }
  return path;
}
