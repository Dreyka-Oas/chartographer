<script lang="ts">
  import * as echarts from "echarts";
  import { onDestroy } from "svelte";

  let { option, height = 320 }: { option: unknown; height?: number } = $props();

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
</script>

<div bind:this={container} style="height: {height}px; width: 100%;"></div>
