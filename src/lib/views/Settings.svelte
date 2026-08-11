<script lang="ts">
  import { api } from "../api";
  import { formatAge, formatDayLong } from "../format";
  import { dashboard } from "../state.svelte";
  import { theme, type ThemeMode } from "../theme.svelte";
  import type { AppErrorPayload, Settings } from "../types";

  /** Valeurs enregistrées, pour détecter ce qui a été modifié depuis. */
  let saved = $state<Settings>({ curseforge_username: null, range_days: 90 });
  let draft = $state<Settings>({ curseforge_username: null, range_days: 90 });
  let unlinked = $state<[number, string, string][]>([]);
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

  // La liste des orphelins est relue après chaque cycle de synchronisation :
  // les appariements automatiques sont recalculés à ce moment-là.
  $effect(() => {
    dashboard.lastSync;
    api
      .unlinkedProjects()
      .then((value) => (unlinked = value))
      .catch(report);
  });

  const dirty = $derived(
    draft.curseforge_username !== saved.curseforge_username || draft.range_days !== saved.range_days,
  );

  async function save() {
    try {
      await api.saveSettings(draft.curseforge_username, draft.range_days);
      saved = { ...draft };
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

  async function link(modrinthId: number, curseforgeId: number) {
    await api.linkManual(modrinthId, curseforgeId);
    unlinked = await api.unlinkedProjects();
    leftId = null;
    rightId = null;
    await dashboard.load();
  }

  const modrinthOrphans = $derived(unlinked.filter(([, platform]) => platform === "modrinth"));
  const curseforgeOrphans = $derived(unlinked.filter(([, platform]) => platform === "curseforge"));
  const orphans = $derived(modrinthOrphans.length + curseforgeOrphans.length);

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
      <h2>Appariements <span class="count">{orphans}</span></h2>
      {#if orphans === 0}
        <div class="row">
          <div class="text">
            <span class="name">Tout est apparié</span>
            <span class="desc">
              Chaque mod présent des deux côtés voit ses téléchargements additionnés.
            </span>
          </div>
        </div>
      {:else}
        <div class="row column">
          <div class="text">
            <span class="name">Projets sans jumeau</span>
            <span class="desc">
              Ces projets n'ont pas trouvé d'équivalent sur l'autre plateforme : soit ils n'y sont
              pas publiés, soit leurs titres diffèrent trop pour être rapprochés automatiquement.
              Sélectionne un projet de chaque colonne pour les apparier à la main.
            </span>
          </div>
          <div class="orphans">
            <div>
              <span class="legend-label">Modrinth · {modrinthOrphans.length}</span>
              {#if modrinthOrphans.length === 0}
                <p class="none">Aucun projet Modrinth en attente.</p>
              {:else}
                <ul>
                  {#each modrinthOrphans as [id, , title] (id)}
                    <li>
                      <button class:active={leftId === id} onclick={() => (leftId = id)}>
                        {title}
                      </button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>
            <div>
              <span class="legend-label">CurseForge · {curseforgeOrphans.length}</span>
              {#if curseforgeOrphans.length === 0}
                <p class="none">
                  Aucun projet CurseForge en attente : les projets ci-contre n'existent
                  vraisemblablement pas sur CurseForge, et resteront comptés sur Modrinth seul.
                </p>
              {:else}
                <ul>
                  {#each curseforgeOrphans as [id, , title] (id)}
                    <li>
                      <button class:active={rightId === id} onclick={() => (rightId = id)}>
                        {title}
                      </button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>
          </div>
          <button
            class="primary"
            disabled={leftId === null || rightId === null}
            onclick={() => {
              if (leftId !== null && rightId !== null) link(leftId, rightId);
            }}
          >
            Apparier les deux sélections
          </button>
        </div>
      {/if}
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
    max-height: 180px;
    overflow-y: auto;
    overscroll-behavior: contain;
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
  .segmented button.active,
  li button.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  li button {
    font-size: 0.78rem;
    padding: 4px 10px;
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
