<script lang="ts">
  import { api } from "../api";
  import CurseforgePoints from "../components/CurseforgePoints.svelte";
  import { formatAge, formatDayLong } from "../format";
  import { dashboard } from "../state.svelte";
  import { theme, type ThemeMode } from "../theme.svelte";
  import type { AppErrorPayload, PairingEntry, Settings } from "../types";

  /** Valeurs enregistrées, pour détecter ce qui a été modifié depuis. */
  const BLANK: Settings = {
    curseforge_username: null,
    range_days: 90,
    currency: "USD",
    curseforge_token_ready: false,
  };
  let saved = $state<Settings>({ ...BLANK });
  let draft = $state<Settings>({ ...BLANK });
  let entries = $state<PairingEntry[]>([]);
  let message = $state("");
  let leftId = $state<number | null>(null);
  let rightId = $state<number | null>(null);
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

  // L'état d'appariement est relu après chaque cycle de synchronisation :
  // les rapprochements automatiques sont recalculés à ce moment-là.
  $effect(() => {
    dashboard.lastSync;
    refreshPairing();
  });

  function refreshPairing() {
    api
      .pairingState()
      .then((value) => (entries = value))
      .catch(report);
  }

  /** Projets d'une plateforme : ceux qui réclament une action d'abord. */
  function column(platform: string) {
    const rank = (e: PairingEntry) => (e.linked_id !== null ? 2 : e.solo ? 1 : 0);
    return entries
      .filter((e) => e.platform === platform)
      .sort((a, b) => rank(a) - rank(b) || a.title.localeCompare(b.title));
  }

  const modrinthList = $derived(column("modrinth"));
  const curseforgeList = $derived(column("curseforge"));
  const pending = $derived(entries.filter((e) => e.linked_id === null && !e.solo).length);

  const leftEntry = $derived(entries.find((e) => e.id === leftId) ?? null);
  const rightEntry = $derived(entries.find((e) => e.id === rightId) ?? null);
  /** Une seule sélection : les actions solo et détacher s'y appliquent. */
  const single = $derived(leftEntry !== null && rightEntry === null ? leftEntry : rightEntry !== null && leftEntry === null ? rightEntry : null);

  async function afterChange() {
    leftId = null;
    rightId = null;
    refreshPairing();
    await dashboard.load();
  }

  async function pair() {
    if (leftId === null || rightId === null) return;
    try {
      await api.linkManual(leftId, rightId);
      await afterChange();
    } catch (e) {
      report(e);
    }
  }

  async function detach(entry: PairingEntry) {
    if (entry.linked_id === null) return;
    const [modrinthId, curseforgeId] =
      entry.platform === "modrinth" ? [entry.id, entry.linked_id] : [entry.linked_id, entry.id];
    try {
      await api.unlink(modrinthId, curseforgeId);
      await afterChange();
    } catch (e) {
      report(e);
    }
  }

  async function toggleSolo(entry: PairingEntry) {
    try {
      await api.setSolo(entry.id, !entry.solo);
      await afterChange();
    } catch (e) {
      report(e);
    }
  }

  function select(entry: PairingEntry) {
    if (entry.platform === "modrinth") {
      leftId = leftId === entry.id ? null : entry.id;
    } else {
      rightId = rightId === entry.id ? null : entry.id;
    }
  }

  const dirty = $derived(
    draft.curseforge_username !== saved.curseforge_username ||
      draft.range_days !== saved.range_days ||
      draft.currency !== saved.currency,
  );

  async function save() {
    try {
      await api.saveSettings(draft.curseforge_username, draft.range_days, draft.currency);
      const changedCurrency = draft.currency !== saved.currency;
      saved = { ...draft };
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
    { id: "appariements", label: "Appariements" },
  ];

  const freshness = $derived(dashboard.overview?.freshness ?? []);
</script>

<div class="layout">
  <aside>
    <h1>Réglages</h1>
    <nav>
      {#each SECTIONS as section (section.id)}
        <a href="#{section.id}">{section.label}</a>
      {/each}
    </nav>
    {#if dashboard.auth?.connected}
      <p class="who">
        <span class="legend-label">Connecté</span>
        <b>{dashboard.auth.username}</b>
      </p>
    {/if}
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
          <span class="name">Pseudo auteur</span>
          <span class="desc">
            Détecté seul en interrogeant CurseForge avec les identifiants de tes projets Modrinth.
            Ne le renseigne que si la détection échoue.
          </span>
        </div>
        <div class="control">
          <input
            value={draft.curseforge_username ?? ""}
            oninput={(e) => (draft.curseforge_username = e.currentTarget.value || null)}
            placeholder="détection automatique"
          />
        </div>
      </div>
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
        <div class="text">
          <span class="name">Compte CurseForge et solde de points</span>
        </div>
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

    <section id="appariements">
      <h2>Appariements <span class="count">{pending}</span></h2>

      <div class="row column">
        <div class="text">
          <span class="name">Rapprocher un mod de son jumeau</span>
          <span class="desc">
            Les deux colonnes listent tous tes projets. Clique un projet à gauche et son équivalent
            à droite, puis Apparier : leurs téléchargements seront additionnés. Un mod publié sur un
            seul site se marque « sans équivalent » — il cesse d'être signalé et reste compté sur sa
            plateforme.
          </span>
        </div>

        <div class="orphans">
          <div>
            <span class="legend-label">Modrinth · {modrinthList.length}</span>
            <ul>
              {#each modrinthList as entry (entry.id)}
                <li>
                  <button
                    class:active={leftId === entry.id}
                    class:linked={entry.linked_id !== null}
                    class:solo={entry.solo}
                    title={entry.linked_to
                      ? `Apparié à ${entry.linked_to}`
                      : entry.solo
                        ? "Déclaré sans équivalent sur CurseForge"
                        : "En attente d'appariement"}
                    onclick={() => select(entry)}
                  >
                    {entry.title}
                  </button>
                </li>
              {/each}
            </ul>
          </div>
          <div>
            <span class="legend-label">CurseForge · {curseforgeList.length}</span>
            {#if curseforgeList.length === 0}
              <p class="none">Aucun projet CurseForge relevé.</p>
            {:else}
              <ul>
                {#each curseforgeList as entry (entry.id)}
                  <li>
                    <button
                      class:active={rightId === entry.id}
                      class:linked={entry.linked_id !== null}
                      class:solo={entry.solo}
                      title={entry.linked_to
                        ? `Apparié à ${entry.linked_to}`
                        : entry.solo
                          ? "Déclaré sans équivalent sur Modrinth"
                          : "En attente d'appariement"}
                      onclick={() => select(entry)}
                    >
                      {entry.title}
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>

        <div class="actions">
          <button class="primary" disabled={leftId === null || rightId === null} onclick={pair}>
            Apparier les deux sélections
          </button>
          {#if single}
            {#if single.linked_id !== null}
              <button onclick={() => detach(single)}>
                Détacher « {single.title} » de {single.linked_to}
              </button>
            {:else}
              <button onclick={() => toggleSolo(single)}>
                {single.solo
                  ? `Remettre « ${single.title} » en attente`
                  : `« ${single.title} » n'existe pas sur l'autre plateforme`}
              </button>
            {/if}
          {/if}
        </div>

        <p class="legend">
          <span class="dot pending"></span> en attente
          <span class="dot linked"></span> apparié
          <span class="dot solo"></span> sans équivalent
        </p>
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
  .who {
    margin: 18px 0 0;
    padding-left: 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.86rem;
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
  .count {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 1px 8px;
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
  .chip {
    font-size: 0.72rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 10px;
    color: var(--text-dim);
    display: inline-flex;
    gap: 6px;
  }
  .chip b {
    font-family: var(--font-mono);
    color: var(--text);
  }
  .chip.ko {
    border-color: var(--error);
    color: var(--error);
  }
  .orphans {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 16px;
  }
  .none {
    margin: 6px 0 0;
    font-size: 0.78rem;
    color: var(--text-dim);
    line-height: 1.45;
  }
  ul {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    max-height: 220px;
    overflow-y: auto;
    overscroll-behavior: contain;
    align-content: flex-start;
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
  input:not([type="number"]) {
    width: 240px;
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
  /*
   * Trois états lisibles d'un coup d'œil : en attente (neutre), apparié (vert),
   * sans équivalent (estompé). La sélection ajoute un cerclage plein.
   */
  li button {
    font-size: 0.78rem;
    padding: 4px 10px;
    color: var(--text);
  }
  li button.linked {
    border-color: var(--modrinth);
    color: var(--modrinth);
  }
  li button.solo {
    color: var(--text-dim);
    border-style: dashed;
  }
  li button.active {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
    color: var(--accent);
  }
  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 0.74rem;
    color: var(--text-dim);
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 3px;
    border: 1px solid var(--border);
    margin-left: 8px;
  }
  .dot:first-child {
    margin-left: 0;
  }
  .dot.linked {
    border-color: var(--modrinth);
    background: var(--modrinth);
  }
  .dot.solo {
    border-style: dashed;
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: var(--accent);
    font-weight: 600;
    align-self: flex-start;
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
