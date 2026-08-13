<script lang="ts">
  /**
   * Point d'interrogation posé à côté d'un intitulé, qui livre son explication
   * au survol.
   *
   * Les précisions de méthode — d'où vient un chiffre, ce qu'il ne dit pas —
   * ont leur place dans l'interface, mais pas en travers du chemin : écrites en
   * clair sous chaque titre, elles prennent plus de place que ce qu'elles
   * commentent. Rangées ici, elles restent à un survol de distance.
   */
  import Tooltip from "./Tooltip.svelte";

  let { text }: { text: string } = $props();
</script>

<Tooltip {text}>
  <button type="button" class="hint" aria-label={text}>?</button>
</Tooltip>

<style>
  .hint {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 15px;
    height: 15px;
    padding: 0;
    background: none;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text-dim);
    font: inherit;
    font-size: 0.62rem;
    line-height: 1;
    cursor: help;
    transition:
      color 140ms ease,
      border-color 140ms ease;
  }
  .hint:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  @media (prefers-reduced-motion: reduce) {
    .hint {
      transition: none;
    }
  }
</style>
