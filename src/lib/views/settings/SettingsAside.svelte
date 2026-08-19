<script lang="ts">
  /** Colonne de gauche : titre, sommaire et comptes reliés. */
  import { api } from "../../api";
  import Tooltip from "../../components/Tooltip.svelte";
  import { dashboard } from "../../state.svelte";
  import type { AppErrorPayload } from "../../types";

  const SECTIONS = [
    { id: "compte", label: "Compte" },
    { id: "synchronisation", label: "Synchronisation" },
    { id: "curseforge", label: "CurseForge" },
    { id: "affichage", label: "Affichage" },
    { id: "mises-a-jour", label: "Mises à jour" },
  ];

  /**
   * Les comptes en service, un par plateforme. Rien n'oblige les deux sites à
   * porter le même pseudo : celui de Modrinth vient du token, celui de
   * CurseForge du tableau de bord relevé.
   */
  const accounts = $derived([
    {
      platform: "Modrinth",
      key: "modrinth",
      name: dashboard.auth?.username ?? null,
      count: dashboard.auth?.modrinth_projects ?? 0,
    },
    {
      platform: "CurseForge",
      key: "curseforge",
      name: dashboard.auth?.curseforge_username ?? null,
      count: dashboard.auth?.curseforge_projects ?? 0,
    },
  ]);

  /** Ouvre la page publique du compte dans le navigateur. L'adresse est bâtie
   * côté application, à partir du seul pseudo. */
  function open(platform: string, name: string | null) {
    if (name === null) return;
    api.openAccountPage(platform, name).catch((e) => {
      dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
    });
  }
</script>

<aside>
  <h1>Réglages</h1>
  <nav>
    {#each SECTIONS as section (section.id)}
      <a href="#{section.id}">{section.label}</a>
    {/each}
  </nav>
  <div class="accounts">
    <span class="legend-label">Comptes reliés</span>
    {#each accounts as account (account.platform)}
      <Tooltip
        block
        text={account.name === null
          ? `Aucun compte ${account.platform} détecté`
          : `Ouvrir la page ${account.platform} de ${account.name}`}
      >
        <button
          class="account {account.key}"
          class:off={account.name === null}
          disabled={account.name === null}
          onclick={() => open(account.key, account.name)}
        >
          <span class="tick"></span>
          <span class="who">
            <b>{account.name ?? "non détecté"}</b>
            <span>{account.platform} · {account.count} projets</span>
          </span>
          <span class="go" aria-hidden="true">↗</span>
        </button>
      </Tooltip>
    {/each}
  </div>
</aside>

<style>
  aside {
    align-self: start;
  }
  h1 {
    font-family: var(--font-display);
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0 0 14px;
  }
  nav {
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
  }
  nav a {
    padding: 6px 12px;
    margin-left: -1px;
    border-left: 2px solid transparent;
    color: var(--text-dim);
    text-decoration: none;
    font-size: 0.84rem;
  }
  nav a:hover {
    color: var(--text);
    border-left-color: var(--accent);
  }
  /*
   * Les deux plateformes n'ont aucune raison de porter le même pseudo : elles
   * s'affichent l'une sous l'autre, chacune avec sa couleur et son compte de
   * projets, pour qu'un nom manquant se voie tout de suite.
   */
  .accounts {
    margin: 20px 0 0;
    padding-left: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  /*
   * Chaque ligne ouvre la page publique du compte : au survol, le filet
   * s'épaissit, la ligne glisse d'un cran et la flèche sort de sa marge —
   * assez pour annoncer un départ vers l'extérieur, sans clignoter.
   */
  .account {
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    padding: 4px 6px 4px 0;
    margin: 0;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      background-color 140ms ease,
      transform 140ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .account:hover:not(:disabled) {
    background-color: var(--surface-2);
    transform: translateX(3px);
  }
  .account:active:not(:disabled) {
    transform: translateX(1px);
  }
  .account:disabled {
    cursor: default;
  }
  .tick {
    width: 3px;
    align-self: stretch;
    min-height: 26px;
    border-radius: 2px;
    transition:
      width 140ms ease,
      opacity 140ms ease;
  }
  .account.modrinth .tick {
    background: var(--modrinth);
  }
  .account.curseforge .tick {
    background: var(--curseforge);
  }
  .account:hover:not(:disabled) .tick {
    width: 5px;
  }
  .account.off .tick {
    background: var(--border);
  }
  .go {
    margin-left: auto;
    font-size: 0.8rem;
    color: var(--text-dim);
    opacity: 0;
    transform: translate(-4px, 2px);
    transition:
      opacity 140ms ease,
      transform 140ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .account:hover:not(:disabled) .go {
    opacity: 1;
    transform: translate(0, 0);
  }
  .account:hover:not(:disabled) .who b {
    color: var(--accent);
  }
  .account.off .go {
    display: none;
  }
  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .who b {
    font-size: 0.86rem;
    overflow-wrap: anywhere;
  }
  .account.off .who b {
    color: var(--text-dim);
    font-weight: 400;
  }
  .who span {
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  @media (prefers-reduced-motion: reduce) {
    .account,
    .account .tick,
    .account .go {
      transition: none;
    }
    .account:hover:not(:disabled) {
      transform: none;
    }
  }
</style>
