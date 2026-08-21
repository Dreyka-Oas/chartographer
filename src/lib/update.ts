/**
 * Ce qu'une mise à jour raconte, sans rien savoir de Tauri.
 *
 * Le module d'à côté tient l'état vivant et parle au socle ; celui-ci ne
 * contient que des fonctions pures, l'avancement, les mots affichés, ce qu'un
 * bouton doit faire. C'est le seul moyen d'éprouver ces règles sans lancer
 * l'application, et donc de les corriger sans avoir à installer une version
 * pour voir le résultat.
 */
import { formatBytes } from "./format";

/**
 * Étapes visibles d'une mise à jour, dans l'ordre où elles se présentent.
 *
 * `idle` n'est pas "rien à signaler" : c'est l'état avant toute question. Il
 * se distingue de `none`, qui est une réponse, le serveur a répondu, cette
 * version est la dernière. Confondre les deux ferait dire à l'interface qu'elle
 * est à jour alors qu'elle n'a encore rien demandé.
 */
export type UpdateStage =
  | "idle"
  | "checking"
  | "none"
  | "available"
  | "downloading"
  | "ready"
  | "error";

/** Progression d'un téléchargement, en octets. Le total peut manquer : tous les
 * serveurs n'annoncent pas la taille du fichier. */
export interface UpdateProgress {
  received: number;
  total: number | null;
}

/**
 * Part téléchargée, de 0 à 1. Nulle tant que la taille reste inconnue : mieux
 * vaut une barre indéterminée qu'une barre qui ment.
 *
 * Le résultat est borné à 1 : un serveur qui annonce une taille plus petite
 * que ce qu'il envoie ne doit pas faire déborder la barre.
 */
export function ratioOf(progress: UpdateProgress): number | null {
  const { received, total } = progress;
  if (total === null || !Number.isFinite(total) || total <= 0) return null;
  if (!Number.isFinite(received) || received < 0) return 0;
  return Math.min(1, received / total);
}

/** Vrai pendant qu'une opération est en cours : la page désactive alors ses
 * boutons plutôt que d'en lancer une seconde par-dessus. */
export function isBusy(stage: UpdateStage): boolean {
  return stage === "checking" || stage === "downloading";
}

/**
 * Ligne d'appoint sous l'intitulé, une phrase par étape.
 *
 * Rend une chaîne vide plutôt que rien tant qu'aucune question n'a été posée :
 * la place de la ligne est ainsi réservée d'avance, et l'arrivée d'une réponse
 * ne déplace pas le bouton d'à côté.
 */
export function noteFor(
  stage: UpdateStage,
  version: string | null,
  progress: UpdateProgress,
  error: string | null,
): string {
  switch (stage) {
    case "idle":
      return "";
    case "checking":
      return "Recherche en cours…";
    case "none":
      return "Cette version est la plus récente.";
    case "available":
      return version === null ? "Une nouvelle version est disponible." : `Version ${version} disponible.`;
    case "downloading": {
      const size = progress.total === null ? "" : ` sur ${formatBytes(progress.total)}`;
      return `Téléchargement : ${formatBytes(progress.received)}${size}.`;
    }
    case "ready":
      return "Installation terminée, l'application va redémarrer.";
    case "error":
      return error ?? "Mise à jour impossible.";
  }
}

/** Ce que le bouton annonce. Il ne propose d'installer qu'une fois la version
 * trouvée : partout ailleurs, il repose la question. */
export function actionLabel(stage: UpdateStage, version: string | null): string {
  switch (stage) {
    case "checking":
      return "Recherche…";
    case "available":
      return version === null ? "Installer" : `Installer ${version}`;
    case "downloading":
      return "Téléchargement…";
    default:
      return "Vérifier maintenant";
  }
}

/**
 * Vrai quand le bouton doit lancer l'installation plutôt qu'une recherche.
 *
 * L'installation ne part que d'un état où une version a été trouvée : ni
 * pendant le téléchargement, ni après une erreur, ni sur une version déjà
 * posée. C'est ce qui garantit qu'aucun binaire n'est remplacé sans qu'on
 * l'ait demandé.
 */
export function shouldInstall(stage: UpdateStage): boolean {
  return stage === "available";
}
