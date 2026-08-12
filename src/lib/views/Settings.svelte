<script lang="ts">
  import { api } from "../api";
  import CurseforgePoints from "../components/CurseforgePoints.svelte";
  import { formatAge, formatDayLong } from "../format";
  import { dashboard } from "../state.svelte";
  import { theme, type ThemeMode } from "../theme.svelte";
  import type { AppErrorPayload, Settings } from "../types";

  /** Valeurs enregistrées, pour détecter ce qui a été modifié depuis. */
  const BLANK: Settings = {
    curseforge_username: null,
    range_days: 90,
    currency: "USD",
    curseforge_token_ready: false,
  };
  let saved = $state<Settings>({ ...BLANK });
  let draft = $state<Settings>({ ...BLANK });
  let message = $state("");
  let loaded = $state(false);

  function report(e: unknown) {
    dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
  }

  $effect(() => {
    if (loaded) return;
    loaded = true;
    api
      .getSettings()
      .then((value) => {
        saved = value;
        draft = { ...value };
      })
      .catch(report);
  });

  const dirty = $derived(draft.range_days !== saved.range_days || draft.currency !== saved.currency);

  async function save() {
    try {
      // Le pseudo CurseForge n'est plus saisi : il se relève tout seul. On
      // repasse celui qui est enregistré pour ne pas l'effacer.
      await api.saveSettings(saved.curseforge_username, draft.range_days, draft.currency);
      const changedCurrency = draft.currency !== saved.currency;
      saved = { ...saved, range_days: draft.range_days, currency: draft.currency };
      // Changer de devise ne veut rien dire sans son taux : on le relève dans
      // la foulée, puis on redessine les montants déjà à l'écran.
      if (changedCurrency) await dashboard.refreshCurrency();
      message = "Réglages enregistrés.";
      // La confirmation s'efface seule : elle n'a rien à faire à l'écran ensuite.
      setTimeout(() => (message = ""), 3000);
    } catch (e) {
      dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
    }
  }

  function revert() {
    draft = { ...saved };
    message = "";
  }

  let capturing = $state(false);

  /**
   * Va chercher le jeton d'envoi sur le compte CurseForge. La fenêtre reste
   * cachée : elle ne s'ouvre que si la session a expiré.
   */
  async function captureToken() {
    capturing = true;
    try {
      const ready = await api.captureCurseforgeToken();
      saved = { ...saved, curseforge_token_ready: ready };
      draft = { ...draft, curseforge_token_ready: ready };
      message = ready
        ? "Jeton d'envoi relevé."
        : "Aucun jeton lisible : reconnecte-toi à CurseForge puis réessaie.";
      setTimeout(() => (message = ""), 4000);
    } catch (e) {
      report(e);
    } finally {
      capturing = false;
    }
  }

  /** Devises proposées. Les deux plateformes paient en dollars ; les autres
   * passent par le taux de référence relevé chaque jour. */
  const CURRENCIES = [
    { code: "USD", label: "Dollar américain ($)" },
    { code: "EUR", label: "Euro (€)" },
    { code: "GBP", label: "Livre sterling (£)" },
    { code: "CHF", label: "Franc suisse (CHF)" },
    { code: "CAD", label: "Dollar canadien ($ CA)" },
    { code: "JPY", label: "Yen (¥)" },
  ];

  const THEMES: { mode: ThemeMode; label: string }[] = [
    { mode: "auto", label: "Automatique" },
    { mode: "light", label: "Clair" },
    { mode: "dark", label: "Sombre" },
  ];

  const SECTIONS = [
    { id: "compte", label: "Compte" },
    { id: "synchronisation", label: "Synchronisation" },
    { id: "curseforge", label: "CurseForge" },
    { id: "affichage", label: "Affichage" },
  ];

  const freshness = $derived(dashboard.overview?.freshness ?? []);

  /**
   * Les comptes en service, un par plateforme. Rien n'oblige les deux sites à
   * porter le même pseudo : celui de Modrinth vient du token, celui de
   * CurseForge du tableau de bord relevé.
   */
  const accounts = $derived([
    {
      platform: "Modrinth",
      name: dashboard.auth?.username ?? null,
      count: dashboard.auth?.modrinth_projects ?? 0,
      tone: "modrinth",
    },
    {
      platform: "CurseForge",
      name: dashboard.auth?.curseforge_username ?? null,
      count: dashboard.auth?.curseforge_projects ?? 0,
      tone: "curseforge",
    },
  ]);
</script>

<div class="layout">
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
        <div class="account" class:off={account.name === null}>
          <span class="tick {account.tone}"></span>
          <span class="who">
            <b>{account.name ?? "non détecté"}</b>
            <span>{account.platform} · {account.count} projets</span>
          </span>
        </div>
      {/each}
    </div>
  </aside>

  <div class="panels">
    <section id="compte">
      <h2>Compte Modrinth</h2>
      {#if dashboard.auth?.connected}
        <div class="row">
          <div class="text">
            <span class="name">Session</span>
            <span class="desc">
              {#if dashboard.auth.connected_since}
                Token enregistré le {formatDayLong(dashboard.auth.connected_since.slice(0, 10))}.
              {/if}
              Il ne quitte jamais cette machine : aucune requête ne part depuis la fenêtre.
            </span>
          </div>
          <div class="control">
            <button onclick={() => api.openTokenPage()}>Gérer mes tokens</button>
            <button class="danger" onclick={() => dashboard.logout()}>Se déconnecter</button>
          </div>
        </div>
      {:else}
        <div class="row">
          <div class="text">
            <span class="name">Aucun token</span>
            <span class="desc">Reconnecte-toi pour relancer les relevés.</span>
          </div>
        </div>
      {/if}
    </section>

    <section id="synchronisation">
      <h2>Synchronisation</h2>
      <div class="row">
        <div class="text">
          <span class="name">Relevé automatique</span>
          <span class="desc">
            Au démarrage puis toutes les six heures. C'est ce rythme qui entretient les snapshots
            quotidiens CurseForge, seule source d'historique de cette plateforme.
          </span>
        </div>
        <div class="control">
          <span class="value">{formatAge(dashboard.dataAgeMs)}</span>
          <button onclick={() => dashboard.sync()} disabled={dashboard.syncing}>
            {dashboard.syncing ? "En cours…" : "Synchroniser"}
          </button>
        </div>
      </div>
      {#if freshness.length > 0}
        <div class="row">
          <div class="text">
            <span class="name">Dernier passage par source</span>
          </div>
          <div class="control sources">
            {#each freshness as entry (entry.provider)}
              <span class="chip" class:ko={entry.status !== "ok"} title={entry.detail}>
                {entry.provider}
                <b>{entry.finished_at ? entry.finished_at.slice(11, 16) : "jamais"}</b>
              </span>
            {/each}
          </div>
        </div>
      {/if}
    </section>

    <section id="curseforge">
      <h2>CurseForge</h2>
      <div class="row">
        <div class="text">
          <span class="name">Jeton d'envoi</span>
          <span class="desc">
            Nécessaire pour publier un fichier. L'application en demande un à ton compte lors de sa
            première collecte, sous le nom « Chartographer », et ne l'affiche jamais. Tu peux le
            révoquer depuis CurseForge à tout moment.
          </span>
        </div>
        <div class="control">
          <span class="value">{saved.curseforge_token_ready ? "en place" : "absent"}</span>
          <button onclick={captureToken} disabled={capturing}>
            {capturing ? "Relevé en cours…" : "Relever"}
          </button>
        </div>
      </div>
      <div class="row column">
        <CurseforgePoints />
      </div>
    </section>

    <section id="affichage">
      <h2>Affichage</h2>
      <div class="row">
        <div class="text">
          <span class="name">Fenêtre par défaut</span>
          <span class="desc">Nombre de jours chargés à l'ouverture de la page de vision.</span>
        </div>
        <div class="control">
          <input type="number" min="7" max="730" bind:value={draft.range_days} />
          <span class="unit">jours</span>
        </div>
      </div>
      <div class="row">
        <div class="text">
          <span class="name">Devise</span>
          <span class="desc">
            Les deux plateformes paient en dollars. Choisir une autre monnaie convertit les montants
            au taux de référence de la Banque centrale européenne, relevé automatiquement.
            {#if dashboard.overview?.currency.day}
              Dernier taux : 1 $ = {dashboard.overview.currency.rate.toFixed(4).replace(".", ",")}
              {dashboard.overview.currency.code}, au {formatDayLong(dashboard.overview.currency.day)}.
            {/if}
          </span>
        </div>
        <div class="control">
          <select bind:value={draft.currency}>
            {#each CURRENCIES as entry (entry.code)}
              <option value={entry.code}>{entry.label}</option>
            {/each}
          </select>
        </div>
      </div>
      <div class="row">
        <div class="text">
          <span class="name">Thème</span>
          <span class="desc">
            En automatique, l'application suit le réglage clair ou sombre de Windows.
          </span>
        </div>
        <div class="control segmented">
          {#each THEMES as option (option.mode)}
            <button class:active={theme.mode === option.mode} onclick={() => theme.set(option.mode)}>
              {option.label}
            </button>
          {/each}
        </div>
      </div>
    </section>
  </div>
</div>

<!-- Barre d'enregistrement : elle n'apparaît que s'il y a quelque chose à enregistrer. -->
{#if dirty || message}
  <div class="bar" class:saved={!dirty}>
    <span>{dirty ? "Modifications non enregistrées" : message}</span>
    {#if dirty}
      <button onclick={revert}>Annuler</button>
      <button class="primary" onclick={save}>Enregistrer</button>
    {/if}
  </div>
{/if}

<style>
  /*
   * Seule la colonne des panneaux défile : le sommaire et le titre restent en
   * place, comme la barre de navigation au-dessus.
   */
  .layout {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
    gap: 28px;
    align-items: stretch;
    height: 100%;
    min-height: 0;
  }
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
  .account {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .tick {
    width: 3px;
    align-self: stretch;
    min-height: 26px;
    border-radius: 2px;
  }
  .tick.modrinth {
    background: var(--modrinth);
  }
  .tick.curseforge {
    background: var(--curseforge);
  }
  .account.off .tick {
    background: var(--border);
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
  .panels {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    padding-right: 4px;
    /* La barre d'enregistrement flotte au-dessus du bas de page. */
    padding-bottom: 64px;
  }
  section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 4px 20px 6px;
    scroll-margin-top: 68px;
  }
  h2 {
    font-family: var(--font-display);
    font-size: 1.05rem;
    font-weight: 600;
    margin: 16px 0 4px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  /* Une rangée de réglage : intitulé à gauche, contrôle à droite, filet entre. */
  .row {
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 16px 0;
    border-top: 1px solid var(--border);
  }
  .row:first-of-type {
    border-top: 0;
  }
  .column {
    flex-direction: column;
    align-items: stretch;
    gap: 14px;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .name {
    font-size: 0.9rem;
    font-weight: 500;
  }
  .desc {
    font-size: 0.8rem;
    color: var(--text-dim);
    max-width: 62ch;
    line-height: 1.45;
  }
  .control {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .value {
    font-family: var(--font-mono);
    font-size: 0.86rem;
    color: var(--text-dim);
  }
  .unit {
    font-size: 0.8rem;
    color: var(--text-dim);
  }
  .sources {
    max-width: 60%;
  }
  /*
   * Le nom de la source et son heure n'ont pas la même fonte : sans hauteur de
   * ligne commune ni centrage, l'heure retombait sous le texte.
   */
  .chip {
    font-size: 0.72rem;
    line-height: 1.6;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 10px;
    color: var(--text-dim);
    display: inline-flex;
    /* Centrer les boîtes ne suffit pas : la chasse fixe n'a pas les mêmes
     * hauteurs de caractère que la fonte d'interface. C'est la ligne de base
     * qu'il faut aligner, comme deux mots d'une même phrase. */
    align-items: baseline;
    gap: 6px;
  }
  .chip b {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    line-height: 1.6;
    color: var(--text);
  }
  .chip.ko {
    border-color: var(--error);
    color: var(--error);
  }
  input {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 8px 10px;
    font: inherit;
    font-size: 0.86rem;
    font-variant-numeric: tabular-nums;
  }
  input[type="number"] {
    width: 92px;
  }
  button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: var(--radius-sm);
    padding: 7px 13px;
    font: inherit;
    font-size: 0.84rem;
    cursor: pointer;
    transition:
      border-color 120ms ease,
      color 120ms ease;
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .danger:hover {
    border-color: var(--error);
    color: var(--error);
  }
  .segmented {
    gap: 4px;
  }
  .segmented button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: var(--accent);
    font-weight: 600;
  }
  .bar {
    position: fixed;
    left: 50%;
    bottom: 20px;
    transform: translateX(-50%);
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px 10px 18px;
    background: var(--surface);
    border: 1px solid var(--accent);
    border-radius: 999px;
    box-shadow: var(--lift);
    font-size: 0.84rem;
    animation: rise 200ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .bar.saved {
    border-color: var(--border);
    color: var(--modrinth);
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 10px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bar {
      animation: none;
    }
  }
</style>
