<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { heatmapOption } from "../charts/heatmap";
  import type { LoaderCell } from "../types";

  let { cells }: { cells: LoaderCell[] } = $props();
  const option = $derived(heatmapOption(cells));
</script>

{#if cells.length === 0}
  <p class="empty">Aucune version indexée. Lance une synchronisation.</p>
{:else}
  <Chart {option} height={300} />
{/if}

<style>
  .empty {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
