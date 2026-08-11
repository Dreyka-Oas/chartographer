<script lang="ts">
  let { stats }: { stats: { label: string; value: string; hint?: string }[] } = $props();
</script>

<div class="row">
  {#each stats as stat, i (stat.label)}
    <article style="animation-delay: {i * 45}ms">
      <span class="legend-label">{stat.label}</span>
      <strong>{stat.value}</strong>
      {#if stat.hint}<span class="hint">{stat.hint}</span>{/if}
    </article>
  {/each}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 12px;
    margin-bottom: 16px;
  }
  article {
    position: relative;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 13px 16px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    animation: rise 320ms cubic-bezier(0.22, 1, 0.36, 1) backwards;
  }
  /* Filet d'accent à gauche : la marge d'un carnet de relevé. */
  article::before {
    content: "";
    position: absolute;
    left: 0;
    top: 12px;
    bottom: 12px;
    width: 2px;
    background: var(--accent);
    opacity: 0.55;
  }
  strong {
    font-family: var(--font-mono);
    font-size: 1.45rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
  }
  .hint {
    font-size: 0.75rem;
    color: var(--text-dim);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    article {
      animation: none;
    }
  }
</style>
