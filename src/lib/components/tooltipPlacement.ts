/**
 * Où poser une bulle d'aide par rapport à ce qu'elle explique.
 *
 * La règle vit à part du composant pour être éprouvée seule : c'est elle qui
 * décide du côté, et c'est elle qui rognait les explications contre le haut de
 * la fenêtre tant que la hauteur de la bulle était supposée au lieu d'être
 * mesurée.
 */

/** Marge entre l'élément et la bulle. */
export const GAP = 8;
/** Garde contre les bords de la vue. */
export const EDGE = 10;

export interface Span {
  top: number;
  bottom: number;
}

export interface Placement {
  /** Position de référence : le bas de la bulle si elle est au-dessus. */
  top: number;
  below: boolean;
}

export function placeBubble(
  anchor: Span,
  height: number,
  viewport: number,
  preferred: "top" | "bottom" = "top",
): Placement {
  const fitsAbove = anchor.top - GAP - height > EDGE;
  const fitsBelow = anchor.bottom + GAP + height < viewport - EDGE;

  // Ni au-dessus ni en dessous : la bulle est plus haute que la place de part
  // et d'autre. On la colle alors sous le bord haut, où elle tient en entier.
  if (!fitsAbove && !fitsBelow) return { top: EDGE + height, below: false };

  const below = preferred === "bottom" ? fitsBelow : !fitsAbove;
  return { below, top: below ? anchor.bottom + GAP : anchor.top - GAP };
}

/**
 * Abscisse du point d'ancrage : le milieu de l'élément, écarté des bords de la
 * moitié de la bulle. Sans cette garde, une bulle collée au bord droit se
 * voyait rognée, le décalage de moitié n'intervenant qu'après la mise en page.
 */
export function anchorX(middle: number, width: number, viewport: number): number {
  const half = Math.min(width, viewport - 2 * EDGE) / 2;
  const room = Math.max(viewport - EDGE - half, EDGE + half);
  return Math.min(Math.max(middle, EDGE + half), room);
}
