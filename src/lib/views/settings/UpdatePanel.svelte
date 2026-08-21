<script lang="ts">
  /**
   * Section "Mises à jour" : version installée, recherche d'une plus récente,
   * téléchargement et relance.
   *
   * L'installation n'est jamais lancée toute seule. Chercher en fond ne coûte
   * rien ; remplacer le binaire pendant qu'on s'en sert demande un accord.
   */
  import Switch from "../../components/Switch.svelte";
  import type { Settings } from "../../types";
  import { actionLabel, noteFor, shouldInstall } from "../../update";
  import { updater } from "../../update.svelte";
  import SettingRow from "./SettingRow.svelte";

  /** Le brouillon est modifié sur place : le parent le tient, et compare. */
  let { draft }: { draft: Settings } = $props();

  const note = $derived(
    noteFor(updater.stage, updater.version, updater.progress, updater.error),
  );
  const label = $derived(actionLabel(updater.stage, updater.version));

  function act() {
    if (shouldInstall(updater.stage)) void updater.install();
    else void updater.check();
  }
</script>

<section id="mises-a-jour" class="stg-panel">
  <h2>Mises à jour</h2>

  <SettingRow
    name="Version installée"
    desc="Les nouvelles versions sont publiées sur GitHub, signées, et vérifiées par l'application avant d'être installées : un fichier qui n'a pas été signé avec la bonne clé est refusé."
    {note}
    noteTone={updater.stage === "error" ? "error" : "plain"}
  >
    {#snippet control()}
      <span class="stg-value">{updater.current ?? "…"}</span>
      <button
        class="stg-btn"
        class:on={updater.stage === "available"}
        onclick={act}
        disabled={updater.busy}
      >
        {label}
      </button>
    {/snippet}
  </SettingRow>

  {#if updater.stage === "downloading"}
    <!-- Barre indéterminée quand le serveur n'annonce pas la taille : une
         barre figée à zéro laisserait croire que rien n'avance. -->
    <div class="bar" class:unknown={updater.ratio === null}>
      <span style:width={updater.ratio === null ? "100%" : `${updater.ratio * 100}%`}></span>
    </div>
  {/if}

  {#if updater.stage === "available" && updater.notes}
    <pre class="notes">{updater.notes}</pre>
  {/if}

  <SettingRow
    name="Chercher au démarrage"
    desc="Une question posée au lancement, en fond. Rien ne s'installe sans un clic ici."
  >
    {#snippet control()}
      <Switch
        bind:checked={draft.auto_update}
        label={draft.auto_update ? "Activé" : "Désactivé"}
        title="Recherche automatique d'une nouvelle version au démarrage"
      />
    {/snippet}
  </SettingRow>
</section>

<style>
  .bar {
    height: 4px;
    border-radius: 999px;
    background: var(--surface-2);
    overflow: hidden;
    margin: 2px 0 6px;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease;
  }
  /*
   * Taille inconnue : la barre balaie sa piste au lieu de se remplir. Le
   * mouvement dit "en cours" sans prétendre à une proportion.
   */
  .bar.unknown span {
    animation: sweep 1200ms ease-in-out infinite;
    transform-origin: left center;
  }
  @keyframes sweep {
    0% {
      transform: scaleX(0.15) translateX(0);
    }
    50% {
      transform: scaleX(0.4) translateX(150%);
    }
    100% {
      transform: scaleX(0.15) translateX(560%);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bar span {
      transition: none;
      animation: none;
    }
  }
  /*
   * Notes de version, telles que la release les porte : Markdown brut, sauts
   * de ligne compris. On ne les met pas en forme, les rendre en HTML voudrait
   * dire faire confiance à un texte venu du réseau.
   */
  .notes {
    margin: 0;
    max-height: 132px;
    overflow-y: auto;
    white-space: pre-wrap;
    font-family: var(--font-mono);
    font-size: 0.74rem;
    line-height: 1.5;
    color: var(--text-dim);
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
  }
</style>
