<script lang="ts">
  /**
   * Demande de confirmation avant une action qui ne se rattrape pas.
   *
   * Le dialogue natif du navigateur ne conviendrait pas : il est dessiné par
   * Windows et ignore le thème. On reprend donc `<dialog>`, qui apporte tout de
   * même l'essentiel — la page passe en arrière-plan inerte, la touche Échap
   * ferme, et le focus reste enfermé dans le dialogue — et on l'habille.
   *
   * Le bouton par défaut est l'annulation : ouvrir ce dialogue puis appuyer sur
   * Entrée sans lire ne doit rien détruire.
   */
  let {
    open = $bindable(false),
    title,
    body = "",
    confirmLabel = "Confirmer",
    cancelLabel = "Annuler",
    danger = false,
    onconfirm,
  }: {
    open: boolean;
    title: string;
    body?: string;
    confirmLabel?: string;
    cancelLabel?: string;
    /** Vrai quand l'action détruit quelque chose : le bouton passe en rouge. */
    danger?: boolean;
    onconfirm: () => void;
  } = $props();

  let element = $state<HTMLDialogElement | null>(null);

  $effect(() => {
    if (!element) return;
    if (open && !element.open) element.showModal();
    if (!open && element.open) element.close();
  });

  function confirm() {
    open = false;
    onconfirm();
  }
</script>

<dialog bind:this={element} onclose={() => (open = false)} oncancel={() => (open = false)}>
  <h2>{title}</h2>
  {#if body}<p>{body}</p>{/if}
  <div class="actions">
    <button class="stg-btn" onclick={() => (open = false)}>{cancelLabel}</button>
    <button class="stg-btn" class:danger class:primary={!danger} onclick={confirm}>
      {confirmLabel}
    </button>
  </div>
</dialog>

<style>
  dialog {
    width: min(420px, calc(100vw - 40px));
    padding: 20px;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--lift);
  }
  dialog::backdrop {
    background: rgba(10, 17, 20, 0.45);
  }
  h2 {
    font-family: var(--font-display);
    font-size: 1.05rem;
    font-weight: 600;
    margin: 0 0 8px;
  }
  p {
    margin: 0;
    font-size: 0.84rem;
    line-height: 1.5;
    color: var(--text-dim);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }
  /*
   * Le bouton destructeur se lit d'emblée comme tel : rouge au repos, et non
   * seulement au survol comme ailleurs dans les réglages.
   */
  .actions .danger {
    box-shadow: inset 0 0 0 1px var(--error);
    color: var(--error);
  }
  .actions .danger:hover {
    background: var(--error);
    color: var(--on-accent);
  }
</style>
