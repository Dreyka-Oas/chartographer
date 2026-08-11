<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { heatmapOption } from "../charts/heatmap";
  import { palette } from "../charts/theme";
  import { theme } from "../theme.svelte";
  import type { LoaderCell } from "../types";

  let { cells }: { cells: LoaderCell[] } = $props();
  const option = $derived(heatmapOption(cells, palette(theme.dark)));
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
