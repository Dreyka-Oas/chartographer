<script lang="ts">
  /**
   * Mot d'état des réglages, posé au bas de la page.
   *
   * Il remplace la barre d'enregistrement d'autrefois : les réglages s'écrivent
   * d'eux-mêmes, il n'y a donc plus rien à valider ni à annuler. Ne restent que
   * deux choses à dire — que l'écriture est en cours, ou qu'elle a eu lieu — et
   * elles s'effacent seules.
   */
  let { text, tone = "plain" }: { text: string; tone?: "plain" | "done" | "error" } = $props();
</script>

{#if text}
  <div class="hint" class:done={tone === "done"} class:error={tone === "error"}>{text}</div>
{/if}

<style>
  .hint {
    position: fixed;
    left: 50%;
    bottom: 20px;
    transform: translateX(-50%);
    z-index: 10;
    padding: 8px 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    box-shadow: var(--lift);
    color: var(--text-dim);
    font-size: 0.82rem;
    /* Rien à cliquer : le mot ne doit pas intercepter la souris. */
    pointer-events: none;
    animation: rise 200ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .hint.done {
    color: var(--modrinth);
  }
  .hint.error {
    border-color: var(--error);
    color: var(--error);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 10px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .hint {
      animation: none;
    }
  }
</style>
