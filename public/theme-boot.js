/**
 * Thème posé avant la première peinture.
 *
 * Appliqué au montage de l'application, il laisserait paraître un éclair de
 * l'autre couleur : la fenêtre s'ouvre avant que le module ne soit évalué.
 *
 * Ce fichier est servi tel quel depuis `public/` au lieu d'être écrit en ligne
 * dans `index.html` : la politique de sécurité de contenu n'autorise aucun
 * script en ligne, et le seul moyen d'en garder un serait d'en épingler
 * l'empreinte à la main, à refaire à chaque virgule changée ici.
 */
try {
  const mode = localStorage.getItem("chartographer:theme");
  if (mode === "light" || mode === "dark") document.documentElement.dataset.theme = mode;
} catch {
  // Stockage indisponible : le thème automatique fera l'affaire.
}
