<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    subtitle = "",
    children,
  }: { title: string; subtitle?: string; children: Snippet } = $props();
</script>

<section>
  <header>
    <h2>{title}</h2>
    {#if subtitle}<p>{subtitle}</p>{/if}
  </header>
  {@render children()}
</section>

<style>
  section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
  }
  header {
    margin-bottom: 12px;
  }
  h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  p {
    margin: 4px 0 0;
    font-size: 0.8rem;
    color: var(--text-dim);
  }
</style>
