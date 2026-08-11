<script lang="ts">
  import { sparklinePath } from "../charts/sparkline";

  let {
    values,
    height = 32,
    color = "var(--accent)",
  }: { values: number[]; height?: number; color?: string } = $props();

  const BOX = { width: 100, height: 30 };
  const path = $derived(sparklinePath(values, BOX.width, BOX.height));
</script>

{#if path.line}
  <svg
    viewBox="0 0 {BOX.width} {BOX.height}"
    preserveAspectRatio="none"
    style="height: {height}px"
    aria-hidden="true"
  >
    <path class="area" d={path.area} fill={color} />
    <path class="line" d={path.line} stroke={color} />
  </svg>
{/if}

<style>
  svg {
    display: block;
    width: 100%;
  }
  .area {
    opacity: 0.16;
  }
  .line {
    fill: none;
    stroke-width: 1.4;
    stroke-linejoin: round;
    /* Sans cela, l'étirement horizontal du viewBox déformerait l'épaisseur. */
    vector-effect: non-scaling-stroke;
  }
</style>
