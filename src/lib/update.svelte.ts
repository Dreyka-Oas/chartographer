import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { isBusy, ratioOf, type UpdateProgress, type UpdateStage } from "./update";

/**
 * État vivant des mises à jour : ce que le socle a répondu, et où en est le
 * téléchargement. Les règles d'affichage, elles, vivent dans `update.ts` —
 * pures, et éprouvées sans lancer l'application.
 *
 * Une question posée au démarrage ne coûte rien ; remplacer le binaire pendant
 * qu'on s'en sert demande un accord. Rien ne s'installe donc sans un clic, et
 * seule une archive signée avec la clé du projet est acceptée — la vérification
 * a lieu dans le socle, avant que quoi que ce soit ne s'exécute.
 */
class Updater {
  stage = $state<UpdateStage>("idle");
  /** Version proposée par le serveur, une fois la question posée. */
  version = $state<string | null>(null);
  /** Version qui tourne en ce moment, relevée à la première question. */
  current = $state<string | null>(null);
  /** Notes de version, telles que la release les porte. */
  notes = $state<string | null>(null);
  progress = $state<UpdateProgress>({ received: 0, total: null });
  error = $state<string | null>(null);

  /**
   * Mise à jour retenue entre la découverte et l'installation. Hors du système
   * réactif : c'est une poignée vers le socle, pas une valeur à afficher.
   */
  private pending: Update | null = null;

  get ratio(): number | null {
    return ratioOf(this.progress);
  }

  get busy(): boolean {
    return isBusy(this.stage);
  }

  /**
   * Interroge le serveur de mises à jour.
   *
   * `silent` sert au démarrage : une panne de réseau au lancement n'a pas à
   * s'afficher en rouge, l'application marche très bien sans avoir posé la
   * question. Le même appel depuis les réglages, lui, doit dire ce qui a
   * échoué — sinon le bouton semble ne rien faire.
   *
   * Le délai d'attente est court et explicite : sans lui, un serveur qui
   * accepte la connexion puis ne répond plus laisserait la recherche pendante
   * indéfiniment, bouton grisé compris.
   */
  async check(silent = false): Promise<void> {
    if (this.busy) return;
    this.stage = "checking";
    this.error = null;
    try {
      if (this.current === null) this.current = await getVersion();
      const found = await check({ timeout: 30_000 });
      if (found === null) {
        await this.forget();
        this.stage = "none";
        return;
      }
      // Une recherche qui en remplace une autre ne doit pas laisser la
      // précédente ouverte côté socle.
      await this.forget();
      this.pending = found;
      this.version = found.version;
      this.notes = found.body ?? null;
      this.stage = "available";
    } catch (e) {
      await this.forget();
      if (silent) {
        // Rien à dire : la question sera reposée au prochain lancement, ou à
        // la main depuis les réglages.
        this.stage = "idle";
        return;
      }
      this.error = message(e);
      this.stage = "error";
    }
  }

  /**
   * Télécharge et installe, puis relance l'application.
   *
   * Le socle vérifie la signature de l'archive avant de l'installer : un
   * fichier qui n'a pas été signé avec la clé du projet fait échouer cet appel,
   * et rien n'est écrit.
   *
   * Sous Windows, l'installateur ferme l'application lui-même : le code qui
   * suit `downloadAndInstall` peut ne jamais s'exécuter. `relaunch` est là pour
   * les plateformes où elle reste ouverte, et l'état `ready` pour le court
   * instant qui les sépare.
   */
  async install(): Promise<void> {
    const update = this.pending;
    if (update === null || this.busy) return;
    this.stage = "downloading";
    this.error = null;
    this.progress = { received: 0, total: null };
    try {
      await update.downloadAndInstall(
        (event) => {
          if (event.event === "Started") {
            this.progress = { received: 0, total: event.data.contentLength ?? null };
          } else if (event.event === "Progress") {
            this.progress = {
              received: this.progress.received + event.data.chunkLength,
              total: this.progress.total,
            };
          } else {
            this.stage = "ready";
          }
        },
        { timeout: 600_000 },
      );
      this.stage = "ready";
      await relaunch();
    } catch (e) {
      this.error = message(e);
      this.stage = "error";
    }
  }

  /**
   * Question posée au démarrage, si le réglage l'autorise. Elle ne bloque
   * rien : l'application s'ouvre pendant que la réponse arrive.
   */
  async boot(enabled: boolean): Promise<void> {
    if (!enabled) {
      if (this.current === null) this.current = await getVersion().catch(() => null);
      return;
    }
    await this.check(true);
  }

  /** Referme la mise à jour tenue côté socle, et oublie ce qu'elle annonçait. */
  private async forget(): Promise<void> {
    const held = this.pending;
    this.pending = null;
    this.version = null;
    this.notes = null;
    if (held !== null) await held.close().catch(() => {});
  }
}

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export const updater = new Updater();
