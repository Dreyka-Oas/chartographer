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
  <header>
    <div class="titles">
      <h2>{title}</h2>
      {#if subtitle}<p>{subtitle}</p>{/if}
    </div>
    {#if onexpand}
      <button onclick={onexpand} title="Ouvrir en plein écran" aria-label="Ouvrir {title}">
        Détail <span aria-hidden="true">↗</span>
      </button>
    {/if}
  </header>

  <!--
    Le contenu occupe toute la hauteur restante : sans cela, une carte étirée par
    sa voisine de rangée laissait une bande vide sous son graphique.
  -->
  <div class="content">
    {@render children()}
  </div>
</section>

<style>
  section {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
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
  header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 16px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .titles {
    min-width: 0;
  }
  .content {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
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
    border-radius: var(--radius-sm);
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
