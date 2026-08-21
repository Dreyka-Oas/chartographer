<script lang="ts">
  import * as echarts from "echarts";
  import { onDestroy } from "svelte";

  /**
   * `height` accepte un nombre de pixels ou "fill" : dans ce cas le graphique
   * occupe toute la hauteur disponible de son conteneur flex, ce qui évite les
   * bandes vides quand une carte est étirée par sa voisine de rangée.
   */
  let {
    option,
    height = 320,
    /**
     * Fait passer le graphique d'un état à l'autre par une transition, au lieu
     * de le redessiner d'un coup.
     *
     * Le remplacement pur et simple (`notMerge`) jette les séries et leurs
     * formes avec : basculer d'un empilement à des courbes superposées sautait
     * à l'image d'arrivée. En remplaçant les seules séries et légendes,
     * ECharts retrouve l'ancienne série par son `id` et interpole entre les
     * deux tracés, ce qui suppose que les options passées portent un `id`
     * stable, sans quoi la série est traitée comme neuve et paraît en fondu.
     *
     * Le reste (axes, zoom) est fusionné, ce qui a le mérite de conserver la
     * plage choisie à la souris quand on change d'affichage.
     */
    morph = false,
  }: { option: unknown; height?: number | "fill"; morph?: boolean } = $props();

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
    chart.setOption(
      option as echarts.EChartsOption,
      morph ? { replaceMerge: ["series", "legend"] } : { notMerge: true },
    );
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
