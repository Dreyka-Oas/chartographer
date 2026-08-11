<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    subtitle = "",
    onexpand = null,
    children,
  }: {
    title: string;
    subtitle?: string;
    /** Fourni, la carte devient cliquable et ouvre sa vue plein écran. */
    onexpand?: (() => void) | null;
    children: Snippet;
  } = $props();
</script>

<section class:clickable={onexpand !== null}>
  <!-- Repères d'angle : les marques de calage d'une planche de carte. -->
  <span class="tick tl" aria-hidden="true"></span>
  <span class="tick br" aria-hidden="true"></span>

  <header>
    <div>
      <h2>{title}</h2>
      {#if subtitle}<p>{subtitle}</p>{/if}
    </div>
    {#if onexpand}
      <button onclick={onexpand} title="Ouvrir en plein écran" aria-label="Ouvrir {title}">
        Détail <span aria-hidden="true">↗</span>
      </button>
    {/if}
  </header>

  {@render children()}
</section>

<style>
  section {
    position: relative;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px 18px 18px;
    transition:
      border-color 140ms ease,
      box-shadow 140ms ease,
      transform 140ms ease;
  }
  .clickable:hover {
    border-color: var(--accent);
    box-shadow: var(--lift);
    transform: translateY(-1px);
  }
  .tick {
    position: absolute;
    width: 9px;
    height: 9px;
    border: 1px solid var(--rule);
    opacity: 0.85;
    pointer-events: none;
  }
  .tl {
    top: -1px;
    left: -1px;
    border-right: 0;
    border-bottom: 0;
  }
  .br {
    right: -1px;
    bottom: -1px;
    border-left: 0;
    border-top: 0;
  }
  header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 14px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--border);
  }
  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 1.02rem;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  p {
    margin: 3px 0 0;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  button {
    margin-left: auto;
    flex-shrink: 0;
    background: none;
    border: 1px solid transparent;
    color: var(--text-dim);
    border-radius: 5px;
    padding: 3px 9px;
    font: inherit;
    font-size: 0.72rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
    transition:
      color 120ms ease,
      border-color 120ms ease;
  }
  button:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
