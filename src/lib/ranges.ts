/**
 * Paliers de durée partagés par les réglages qui parlent de jours.
 *
 * La fenêtre glissante de la barre de filtres et la fenêtre de comparaison du
 * classement des journées parlent toutes deux de durées comparables, "les
 * N derniers jours", et l'utilisateur s'attend à retrouver les mêmes
 * paliers des deux côtés plutôt que deux découpages qui ne se recoupent qu'à
 * moitié. Une seule liste, deux lecteurs.
 */
export const RANGES = [30, 90, 180, 365];
