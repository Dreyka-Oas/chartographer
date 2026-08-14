<script lang="ts">
  import type { Snippet } from "svelte";

  import Tooltip from "./Tooltip.svelte";

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
      <Tooltip text="Ouvrir en plein écran">
        <button onclick={onexpand} aria-label="Ouvrir {title}">
          Détail <span aria-hidden="true">↗</span>
        </button>
      </Tooltip>
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
      box-shadow 140ms ease;
  }
  /* La carte ne bouge pas au survol : le liseré et l'ombre suffisent à dire
   * qu'elle s'ouvre, sans faire sauter le contenu sous le curseur. */
  .clickable:hover {
    border-color: var(--accent);
    box-shadow: var(--lift);
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
  /* Les titres poussent le bouton de détail contre le bord droit. Un
   * `margin-left: auto` sur le bouton ne le ferait plus : il est enveloppé
   * d'une bulle d'aide, et n'est donc plus l'enfant direct de l'en-tête. */
  .titles {
    flex: 1;
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
