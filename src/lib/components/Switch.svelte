<script lang="ts">
  /**
   * Interrupteur d'affichage.
   *
   * Une case à cocher annonce un formulaire ; ici rien n'est enregistré, on
   * bascule une vue. Le curseur glisse, la piste prend la couleur d'accent, et
   * l'intitulé passe en pleine encre une fois enclenché.
   */
  import Tooltip from "./Tooltip.svelte";

  let {
    checked = $bindable(false),
    label,
    title = "",
  }: { checked?: boolean; label: string; title?: string } = $props();
</script>

<Tooltip text={title}>
  <button
    type="button"
    class="toggle"
    class:on={checked}
    role="switch"
    aria-checked={checked}
    onclick={() => (checked = !checked)}
  >
    <span class="track"><span class="knob"></span></span>
    {label}
  </button>
</Tooltip>

<style>
  .toggle {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px 4px 4px;
    background: none;
    border: 1px solid transparent;
    border-radius: 999px;
    font: inherit;
    font-size: 0.8rem;
    color: var(--text-dim);
    cursor: pointer;
    transition:
      color 140ms ease,
      border-color 140ms ease;
  }
  .toggle:hover {
    color: var(--text);
    border-color: var(--border);
  }
  .toggle.on {
    color: var(--text);
  }
  .track {
    width: 30px;
    height: 17px;
    flex: none;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    display: inline-flex;
    align-items: center;
    padding: 0 2px;
    transition:
      background-color 160ms ease,
      border-color 160ms ease;
  }
  .toggle.on .track {
    background: var(--accent);
    border-color: var(--accent);
  }
  .knob {
    width: 11px;
    height: 11px;
    border-radius: 999px;
    background: var(--text-dim);
    transition:
      transform 180ms cubic-bezier(0.22, 1, 0.36, 1),
      background-color 160ms ease;
  }
  .toggle.on .knob {
    background: var(--on-accent);
    transform: translateX(13px);
  }
  @media (prefers-reduced-motion: reduce) {
    .toggle,
    .track,
    .knob {
      transition: none;
    }
  }
</style>
