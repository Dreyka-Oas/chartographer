/** État d'une étape du démarrage, dans l'ordre où il peut évoluer. */
export type StepState = "waiting" | "running" | "done" | "failed";

export interface BootStep {
  key: string;
  label: string;
  state: StepState;
  /** Ce que l'étape a rapporté, montré sous son intitulé une fois close. */
  note: string;
}

/**
 * Ce que le démarrage joue, dans l'ordre.
 *
 * Les cinq clés du milieu portent les noms rendus par le cycle Rust : l'écran
 * suit ainsi la synchronisation réelle, sans table de correspondance à tenir à
 * jour de part et d'autre.
 */
export const PLAN: { key: string; label: string }[] = [
  { key: "comptes", label: "Comptes reliés" },
  { key: "reglages", label: "Préférences" },
  { key: "modrinth", label: "Projets Modrinth" },
  { key: "curseforge", label: "Projets CurseForge" },
  { key: "matching", label: "Appariement des mods" },
  { key: "modrinth-analytics", label: "Statistiques quotidiennes" },
  { key: "curseforge-snapshot", label: "Relevés CurseForge" },
  { key: "tableau", label: "Tableau de bord auteur" },
  { key: "agregation", label: "Mise en page des chiffres" },
];

/** Les étapes du plan que le cycle Rust joue lui-même, et lui seul. */
export const SYNC_KEYS = [
  "modrinth",
  "curseforge",
  "matching",
  "modrinth-analytics",
  "curseforge-snapshot",
];

export function freshSteps(): BootStep[] {
  return PLAN.map((step) => ({ ...step, state: "waiting" as StepState, note: "" }));
}

/**
 * Avancement de 0 à 1.
 *
 * Une étape en cours compte pour moitié : la barre avance dès qu'un travail
 * commence, sans jamais donner pour acquis ce qui n'est pas terminé. Les cinq
 * relevés du cycle durent chacun plusieurs secondes, et une barre figée
 * pendant tout ce temps se lit comme une application bloquée.
 */
export function progressOf(steps: BootStep[]): number {
  if (steps.length === 0) return 1;
  const weight = (state: StepState) => (state === "waiting" ? 0 : state === "running" ? 0.5 : 1);
  return steps.reduce((sum, step) => sum + weight(step.state), 0) / steps.length;
}

/** Intitulé de l'étape en cours, ou la dernière close quand rien ne tourne. */
export function currentLabel(steps: BootStep[]): string {
  const running = steps.find((step) => step.state === "running");
  if (running) return running.label;
  const closed = steps.filter((step) => step.state === "done" || step.state === "failed");
  return closed.length === 0 ? PLAN[0].label : closed[closed.length - 1].label;
}
