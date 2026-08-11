<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { timelineOption } from "../charts/timeline";
  import { palette } from "../charts/theme";
  import { theme } from "../theme.svelte";
  import type { TimelinePoint } from "../types";

  let { points }: { points: TimelinePoint[] } = $props();
  let stacked = $state(true);
  const option = $derived(timelineOption(points, stacked, palette(theme.dark)));
</script>

<label>
  <input type="checkbox" bind:checked={stacked} />
  Empiler les plateformes
</label>
<Chart {option} height={340} />

<style>
  label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: var(--text-dim);
    margin-bottom: 8px;
  }
</style>
