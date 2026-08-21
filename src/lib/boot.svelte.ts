import { type BootStep, currentLabel, freshSteps, progressOf } from "./boot";
import type { SyncReport, SyncStep } from "./types";

/**
 * Avancement du démarrage, tel que l'écran d'ouverture le montre.
 *
 * L'application relève tout avant de s'afficher : les chiffres d'une page qui
 * paraît d'abord vide, puis se remplit par morceaux, se lisent mal, on ne sait
 * jamais si un total est bas ou seulement pas encore arrivé.
 */
class Boot {
  steps = $state<BootStep[]>(freshSteps());
  /**
   * Vrai quand la page peut paraître : mise en route terminée, ou comptes
   * manquants, c'est alors l'écran de connexion qui prend la suite.
   */
  done = $state(false);

  get progress(): number {
    return progressOf(this.steps);
  }

  get label(): string {
    return currentLabel(this.steps);
  }

  private find(key: string): BootStep | undefined {
    return this.steps.find((step) => step.key === key);
  }

  /** Ouvre une étape. Sans effet une fois la page rendue : le même cycle sert
   * ensuite aux relevés périodiques, qui n'ont plus d'écran à renseigner. */
  open(key: string) {
    if (this.done) return;
    const step = this.find(key);
    if (step) step.state = "running";
  }

  close(key: string, ok: boolean, note = "") {
    if (this.done) return;
    const step = this.find(key);
    if (!step) return;
    step.state = ok ? "done" : "failed";
    step.note = note;
  }

  /** Traduit un jalon du cycle Rust : les clés sont les mêmes des deux côtés. */
  fromSync(step: SyncStep) {
    if (step.report === null) this.open(step.provider);
    else this.close(step.provider, step.report.status === "ok", step.report.detail);
  }

  /**
   * Applique les comptes rendus finaux du cycle. Les jalons arrivent en direct
   * quand l'application tourne dans sa fenêtre ; hors de là, ces rapports sont
   * la seule chose qui parvienne à l'écran.
   */
  adopt(reports: SyncReport[]) {
    for (const report of reports) {
      const step = this.find(report.provider);
      if (step?.state === "waiting" || step?.state === "running") {
        this.close(report.provider, report.status === "ok", report.detail);
      }
    }
  }

  /**
   * Clôt ce qui traîne parmi les clés données. Un cycle qui rend la main sans
   * avoir posé tous ses jalons laisserait sinon des étapes en suspens, et
   * l'écran attendrait indéfiniment une nouvelle qui ne viendra pas.
   */
  settle(keys: string[], ok: boolean, note = "") {
    for (const key of keys) {
      const step = this.find(key);
      if (step && (step.state === "waiting" || step.state === "running")) {
        this.close(key, ok, note);
      }
    }
  }

  /** Laisse la page paraître ; l'écran de démarrage s'efface par-dessus. */
  release() {
    this.done = true;
  }

  /** Remet tout à zéro : une déconnexion rejouera le démarrage en entier. */
  reset() {
    this.steps = freshSteps();
    this.done = false;
  }
}

export const boot = new Boot();
