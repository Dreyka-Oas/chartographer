<script lang="ts">
  import { BRANDS, type PlatformName } from "./brands";

  let {
    platform,
    account = null,
    count = 0,
    active = true,
    ontoggle,
  }: {
    platform: PlatformName;
    /** Pseudo du compte relevé sur cette plateforme, s'il est connu. */
    account?: string | null;
    /** Nombre de projets relevés : zéro signifie que rien n'a été trouvé. */
    count?: number;
    /** Faux quand la plateforme est masquée de l'affichage. */
    active?: boolean;
    ontoggle?: () => void;
  } = $props();

  const brand = $derived(BRANDS[platform]);
  const connected = $derived(count > 0);
  const title = $derived(
    !connected
      ? `${brand.label} · aucun projet relevé`
      : `${brand.label}${account ? ` · ${account}` : ""} · ${count} projet${count > 1 ? "s" : ""}` +
        (active ? " · cliquer pour masquer" : " · masqué, cliquer pour réafficher"),
  );
</script>

<button
  type="button"
  class="badge {platform}"
  class:off={!connected}
  class:muted={connected && !active}
  disabled={!connected}
  aria-pressed={active}
  {title}
  onclick={() => ontoggle?.()}
>
  <svg viewBox="0 0 24 24" aria-hidden="true"><path d={brand.path} /></svg>
  <b>{brand.label}</b>
</button>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px 3px 8px;
    background: none;
    border: 1px solid currentColor;
    border-radius: 999px;
    font: inherit;
    font-size: 0.72rem;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition:
      opacity 120ms ease,
      color 120ms ease;
  }
  .badge:disabled {
    cursor: default;
  }
  .modrinth {
    color: var(--modrinth);
  }
  .curseforge {
    color: var(--curseforge);
  }
  /* Aucun projet relevé : la plateforme s'efface au lieu de mentir sur son état. */
  .off {
    color: var(--text-dim);
    border-style: dashed;
    opacity: 0.7;
  }
  /* Masquée par l'utilisateur : la pastille reste colorée mais s'estompe. */
  .muted {
    opacity: 0.42;
    border-style: dashed;
  }
  .badge:hover:not(:disabled) {
    opacity: 1;
    filter: brightness(1.1);
  }
  svg {
    width: 13px;
    height: 13px;
    fill: currentColor;
  }
  b {
    font-weight: 500;
  }
</style>
