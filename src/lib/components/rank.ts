/**
 * Couleurs du podium d'un classement.
 *
 * Or, argent, bronze : ces teintes sont écrites en clair plutôt que prises au
 * thème, parce qu'elles ne veulent rien dire d'autre qu'un rang et doivent se
 * lire pareil en clair comme en sombre.
 */
export const PODIUM = ["#d4a72c", "#9aa4ad", "#b97a45"];

/** La couleur du rang, ou `null` au-delà des trois premiers. */
export function podiumColor(index: number): string | null {
  return PODIUM[index] ?? null;
}

/** Une colonne de `RankedTable`. */
export interface Column {
  label: string;
  /** Les intitulés sont à droite comme les chiffres, sauf mention. */
  align?: "left" | "right";
}
