<script lang="ts">
  /**
   * Pastille d'état d'une plateforme. Le pictogramme est un monogramme dessiné
   * ici, aux couleurs de la plateforme : ce n'est pas le logo officiel, que
   * l'application n'a pas à redistribuer.
   */
  let {
    platform,
    label,
    account = null,
    count = 0,
  }: {
    platform: "modrinth" | "curseforge";
    label: string;
    /** Pseudo du compte relevé sur cette plateforme, s'il est connu. */
    account?: string | null;
    /** Nombre de projets relevés : zéro signifie non connecté. */
    count?: number;
  } = $props();

  const connected = $derived(count > 0);
  const title = $derived(
    connected
      ? `${label}${account ? ` · ${account}` : ""} · ${count} projet${count > 1 ? "s" : ""}`
      : `${label} · aucun projet relevé`,
  );
</script>

<span class="badge {platform}" class:off={!connected} {title}>
  <svg viewBox="0 0 16 16" aria-hidden="true">
    {#if platform === "modrinth"}
      <!-- Anneau ouvert : la boucle du monogramme Modrinth. -->
      <path d="M8 1.6a6.4 6.4 0 1 0 6.2 8" />
      <path d="M4.6 8.6 7 6.2l1.9 1.9 1.5-1.5" />
    {:else}
      <!-- Chevron ascendant : la flèche du monogramme CurseForge. -->
      <path d="M2.4 11.4 6 4.6l3.2 4.2 1.6-1.9 2.8 4.5" />
      <path d="M2.4 13.6h11.2" />
    {/if}
  </svg>
  <b>{label}</b>
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 9px 3px 7px;
    border: 1px solid currentColor;
    border-radius: 999px;
    font-size: 0.72rem;
    letter-spacing: 0.02em;
  }
  .modrinth {
    color: var(--modrinth);
  }
  .curseforge {
    color: var(--curseforge);
  }
  /* Sans projet relevé, la plateforme s'efface au lieu de mentir sur son état. */
  .off {
    color: var(--text-dim);
    border-style: dashed;
    opacity: 0.7;
  }
  svg {
    width: 13px;
    height: 13px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  b {
    font-weight: 500;
  }
</style>
