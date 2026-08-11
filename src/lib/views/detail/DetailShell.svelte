<script lang="ts">
  import type { Snippet } from "svelte";
  import { dashboard } from "../../state.svelte";

  let {
    title,
    subtitle = "",
    icon = null,
    actions,
    children,
  }: {
    title: string;
    subtitle?: string;
    icon?: string | null;
    actions?: Snippet;
    children: Snippet;
  } = $props();

  const RANGES = [30, 90, 180, 365];

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") dashboard.closeDetail();
  }
</script>

<svelte:window {onkeydown} />

<section class="shell">
  <header>
    <button class="back" onclick={() => dashboard.closeDetail()} title="Échap pour fermer">
      <span aria-hidden="true">←</span> Retour
    </button>

    <div class="identity">
      {#if icon}<img src={icon} alt="" />{/if}
      <div>
        <h1>{title}</h1>
        {#if subtitle}<p>{subtitle}</p>{/if}
      </div>
    </div>

    <div class="tools">
      {#if actions}{@render actions()}{/if}
      <div class="ranges">
        {#each RANGES as days (days)}
          <button
            class:active={dashboard.rangeDays === days}
            onclick={() => dashboard.setRange(days)}
          >
            {days} j
          </button>
        {/each}
      </div>
    </div>
  </header>

  <div class="body">
    {@render children()}
  </div>
</section>

<style>
  .shell {
    position: fixed;
    inset: 0;
    z-index: 20;
    background: var(--bg);
    display: flex;
    flex-direction: column;
    animation: sheet 260ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes sheet {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .shell {
      animation: none;
    }
  }
  header {
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 14px 24px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    flex-shrink: 0;
  }
  .identity {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .identity img {
    width: 38px;
    height: 38px;
    border-radius: 8px;
  }
  h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.25rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  header p {
    margin: 2px 0 0;
    font-size: 0.8rem;
    color: var(--text-dim);
  }
  .tools {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .ranges {
    display: flex;
    gap: 4px;
  }
  button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 7px;
    padding: 5px 12px;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
    white-space: nowrap;
  }
  button:hover,
  button.active {
    color: var(--text);
    border-color: var(--accent);
  }
  .back {
    font-weight: 500;
  }
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 20px 24px 40px;
  }
</style>
