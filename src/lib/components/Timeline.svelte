<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { palette } from "../charts/theme";
  import { timelineOption } from "../charts/timeline";
  import { dashboard } from "../state.svelte";
  import { theme } from "../theme.svelte";
  import type { TimelinePoint } from "../types";

  let { points }: { points: TimelinePoint[] } = $props();
  let stacked = $state(true);
  const option = $derived(timelineOption(points, stacked, palette(theme.dark)));

  /** Rien à tracer : toutes les journées de la fenêtre sont à zéro. */
  const empty = $derived(points.every((p) => p.modrinth + p.curseforge === 0));
  const onlyCurseforge = $derived(!dashboard.platforms.modrinth);
</script>

{#if empty}
  <p class="empty">
    {#if onlyCurseforge}
      CurseForge ne publie aucun historique de téléchargements. Chartographer le reconstruit en
      comparant deux relevés successifs, à raison d'un par jour : tant qu'un seul jour est
      enregistré, il n'y a aucun écart à tracer. La première courbe apparaîtra au relevé de demain.
      Les totaux cumulés du tableau, eux, sont exacts dès maintenant.
    {:else}
      Aucun téléchargement relevé sur cette période.
    {/if}
  </p>
{:else}
  <label>
    <input type="checkbox" bind:checked={stacked} />
    Empiler les plateformes
  </label>
  <Chart {option} height="fill" />
{/if}

<style>
  label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: var(--text-dim);
    margin-bottom: 8px;
  }
  .empty {
    margin: 0;
    align-self: center;
    max-width: 52ch;
    color: var(--text-dim);
    font-size: 0.84rem;
    line-height: 1.55;
  }
</style>
