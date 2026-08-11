<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { revenueOption } from "../charts/revenue";
  import type { RevenuePoint } from "../types";

  let { points }: { points: RevenuePoint[] } = $props();
  const option = $derived(revenueOption(points));
</script>

{#if points.length === 0}
  <p class="empty">Aucun revenu sur la période.</p>
{:else}
  <Chart {option} height={280} />
{/if}

<style>
  .empty {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0;
  }
</style>
