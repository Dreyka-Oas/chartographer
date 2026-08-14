/**
 * La grille d'un mois, en dates ISO.
 *
 * Le calcul vit à part du composant pour être vérifié sans DOM : les mois qui
 * débordent, les semaines à cheval et les années bissextiles sont exactement
 * ce qu'un calendrier rate, et ce sont des questions de calcul, pas d'affichage.
 *
 * Tout passe par l'UTC. En heure locale, une date construite à minuit peut
 * reculer d'un jour au passage à l'heure d'été, et la grille se décale d'une
 * case pour la moitié de l'année.
 */

/** Six semaines pleines : la hauteur du panneau ne saute pas d'un mois à l'autre. */
const WEEKS = 6;

function utc(year: number, month: number, day: number): Date {
  return new Date(Date.UTC(year, month, day));
}

function iso(date: Date): string {
  return date.toISOString().slice(0, 10);
}

/**
 * Les six semaines qui couvrent `month` (`YYYY-MM`), lundi en tête.
 *
 * Les cases des mois voisins sont rendues avec les autres : le composant les
 * grise, mais elles restent cliquables — c'est ainsi qu'on passe au mois suivant
 * sans viser la flèche.
 */
export function monthGrid(month: string, weekStartsOn = 1): string[][] {
  const [year, index] = month.split("-").map(Number);
  const first = utc(year, index - 1, 1);
  // `getUTCDay` rend 0 pour dimanche : le décalage ramène le premier jour de la
  // semaine choisie en tête, sans jamais passer en négatif.
  const lead = (first.getUTCDay() - weekStartsOn + 7) % 7;
  const start = utc(year, index - 1, 1 - lead);

  return Array.from({ length: WEEKS }, (_, week) =>
    Array.from({ length: 7 }, (_, day) =>
      iso(utc(start.getUTCFullYear(), start.getUTCMonth(), start.getUTCDate() + week * 7 + day)),
    ),
  );
}

/** Déplace un mois `YYYY-MM` de `by` mois, année comprise. */
export function shiftMonth(month: string, by: number): string {
  const [year, index] = month.split("-").map(Number);
  const moved = utc(year, index - 1 + by, 1);
  return `${moved.getUTCFullYear()}-${String(moved.getUTCMonth() + 1).padStart(2, "0")}`;
}

/**
 * Décale une date ISO de `delta` jours, en passant par l'UTC pour la même
 * raison que `monthGrid` : la navigation au clavier ne doit pas sauter un jour
 * au changement d'heure.
 */
export function addDays(day: string, delta: number): string {
  const [year, month, date] = day.split("-").map(Number);
  return iso(utc(year, month - 1, date + delta));
}
