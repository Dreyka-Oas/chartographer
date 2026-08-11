<script lang="ts">
  import * as echarts from "echarts";
  import { onDestroy } from "svelte";

  /**
   * `height` accepte un nombre de pixels ou "fill" : dans ce cas le graphique
   * occupe toute la hauteur disponible de son conteneur flex, ce qui évite les
   * bandes vides quand une carte est étirée par sa voisine de rangée.
   */
  let { option, height = 320 }: { option: unknown; height?: number | "fill" } = $props();

  let container = $state<HTMLDivElement | null>(null);
  let chart: echarts.ECharts | null = null;
  let observer: ResizeObserver | null = null;

  $effect(() => {
    if (!container) return;
    if (!chart) {
      chart = echarts.init(container, null, { renderer: "canvas" });
      observer = new ResizeObserver(() => chart?.resize());
      observer.observe(container);
    }
    chart.setOption(option as echarts.EChartsOption, { notMerge: true });
  });

  onDestroy(() => {
    observer?.disconnect();
    chart?.dispose();
    chart = null;
  });

  const style = $derived(
    height === "fill"
      ? "flex: 1; min-height: 220px; width: 100%;"
      : `height: ${height}px; width: 100%;`,
  );
</script>

<div bind:this={container} {style}></div>
