<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import Hint from "./Hint.svelte";
  import Switch from "./Switch.svelte";
  import { palette } from "../charts/theme";
  import { timelineOption } from "../charts/timeline";
  import { dashboard } from "../state.svelte";
  import { theme } from "../theme.svelte";
  import type { TimelinePoint } from "../types";

  let { points }: { points: TimelinePoint[] } = $props();
  // Coché par défaut : la première lecture attendue est le total des deux
  // plateformes. Décocher les sépare, pour les comparer niveau à niveau.
  let stacked = $state(true);
  const option = $derived(timelineOption(points, stacked, palette(theme.dark)));

  /** Rien à tracer : toutes les journées de la fenêtre sont à zéro. */
  const empty = $derived(points.every((p) => p.modrinth + p.curseforge === 0));
  const onlyCurseforge = $derived(!dashboard.platforms.modrinth);
</script>

{#if empty}
  <p class="empty">
    Aucun téléchargement relevé sur cette période.
    {#if onlyCurseforge}
      <Hint
        text="CurseForge ne publie aucun historique de téléchargements. Chartographer le reconstruit en comparant deux relevés successifs, à raison d'un par jour : tant qu'un seul jour est enregistré, il n'y a aucun écart à tracer. La première courbe apparaîtra au relevé de demain. Les totaux cumulés du tableau, eux, sont exacts dès maintenant."
      />
    {/if}
  </p>
{:else}
  <div class="head">
    <Switch
      bind:checked={stacked}
      label="Empiler les plateformes"
      title={stacked
        ? "Les deux plateformes s'additionnent : la courbe montre le total"
        : "Les deux plateformes se superposent : les niveaux se comparent"}
    />
  </div>
  <Chart {option} height="fill" morph />
{/if}

<style>
  .head {
    margin-bottom: 8px;
  }
  .empty {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    align-self: center;
    max-width: 52ch;
    color: var(--text-dim);
    font-size: 0.84rem;
    line-height: 1.55;
  }
</style>
