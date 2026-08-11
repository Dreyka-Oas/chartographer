<script lang="ts">
  import { api } from "../api";
  import { formatAge } from "../format";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, Settings } from "../types";

  let settings = $state<Settings>({ curseforge_username: null, range_days: 90 });
  let unlinked = $state<[number, string, string][]>([]);
  let message = $state("");
  let leftId = $state<number | null>(null);
  let rightId = $state<number | null>(null);
  let loaded = $state(false);

  $effect(() => {
    if (loaded) return;
    loaded = true;
    api.getSettings().then((value) => (settings = value));
    api.unlinkedProjects().then((value) => (unlinked = value));
  });

  async function save() {
    try {
      await api.saveSettings(settings.curseforge_username, settings.range_days);
      message = "Réglages enregistrés.";
    } catch (e) {
      dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
    }
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
</script>

<header class="page">
  <div>
    <h1>Réglages</h1>
    <p class="hint">Compte, détection CurseForge, fenêtre par défaut et appariements.</p>
  </div>
  <button class="primary" onclick={save}>Enregistrer</button>
</header>

{#if message}<p class="ok">{message}</p>{/if}

<div class="grid">
  <section>
    <h2>Compte Modrinth</h2>
    {#if dashboard.auth?.connected}
      <p>
        Connecté en tant que <b>{dashboard.auth.username}</b> depuis
        {dashboard.auth.connected_since?.slice(0, 10)}.
      </p>
      <div class="pair">
        <button onclick={() => dashboard.logout()}>Se déconnecter</button>
        <button onclick={() => api.openTokenPage()}>Gérer mes tokens</button>
      </div>
    {:else}
      <p class="hint">Aucun token enregistré.</p>
    {/if}
  </section>

  <section>
    <h2>Synchronisation</h2>
    <p class="hint">
      Automatique : l'application se resynchronise au démarrage puis toutes les six heures, ce qui
      entretient les snapshots quotidiens CurseForge.
    </p>
    <dl>
      <dt>Dernier relevé</dt>
      <dd>{formatAge(dashboard.dataAgeMs)}</dd>
    </dl>
    <button onclick={() => dashboard.sync()} disabled={dashboard.syncing}>
      {dashboard.syncing ? "Synchronisation…" : "Synchroniser maintenant"}
    </button>
  </section>

  <section>
    <h2>CurseForge</h2>
    <p class="hint">
      Détecté automatiquement depuis tes projets Modrinth. Renseigne-le seulement si la détection
      échoue.
    </p>
    <input
      value={settings.curseforge_username ?? ""}
      oninput={(e) => (settings.curseforge_username = e.currentTarget.value || null)}
      placeholder="pseudo auteur CurseForge"
    />
  </section>

  <section>
    <h2>Fenêtre d'historique</h2>
    <p class="hint">Nombre de jours affichés à l'ouverture de la page de vision.</p>
    <div class="pair">
      <input type="number" min="7" max="730" bind:value={settings.range_days} />
      <span class="hint unit">jours</span>
    </div>
  </section>

  <section class="wide">
    <h2>Appariements manquants <span class="count">{orphans}</span></h2>
    {#if orphans === 0}
      <p class="hint">Tous les projets sont appariés.</p>
    {:else}
      <p class="hint">
        Ces projets n'ont pas trouvé leur jumeau sur l'autre plateforme. Rapproche-les à la main pour
        que leurs téléchargements soient additionnés.
      </p>
      <div class="pair">
        <select bind:value={leftId}>
          <option value={null}>Projet Modrinth…</option>
          {#each modrinthOrphans as [id, , title] (id)}<option value={id}>{title}</option>{/each}
        </select>
        <select bind:value={rightId}>
          <option value={null}>Projet CurseForge…</option>
          {#each curseforgeOrphans as [id, , title] (id)}<option value={id}>{title}</option>{/each}
        </select>
        <button
          disabled={leftId === null || rightId === null}
          onclick={() => {
            if (leftId !== null && rightId !== null) link(leftId, rightId);
          }}
        >
          Apparier
        </button>
      </div>
    {/if}
  </section>
</div>

<style>
  .page {
    display: flex;
    align-items: flex-end;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 16px;
  }
  h1 {
    font-family: var(--font-display);
    font-size: 1.5rem;
    font-weight: 600;
    margin: 0;
  }
  .page .primary {
    margin-left: auto;
  }
  h2 {
    font-family: var(--font-display);
    font-size: 1rem;
    font-weight: 600;
    margin: 0 0 10px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
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
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
    gap: 16px;
    align-items: start;
  }
  section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px 18px 18px;
  }
  .wide {
    grid-column: 1 / -1;
  }
  p {
    margin: 0 0 12px;
    font-size: 0.86rem;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.8rem;
  }
  .unit {
    align-self: center;
  }
  dl {
    display: flex;
    gap: 10px;
    align-items: baseline;
    margin: 0 0 12px;
  }
  dt {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-dim);
  }
  dd {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 0.9rem;
  }
  input,
  select {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 8px 10px;
    font: inherit;
    font-size: 0.86rem;
    max-width: 100%;
  }
  button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: var(--radius-sm);
    padding: 8px 14px;
    font: inherit;
    font-size: 0.86rem;
    cursor: pointer;
    transition: border-color 120ms ease;
  }
  button:hover:not(:disabled) {
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border: 0;
    font-weight: 600;
  }
  .pair {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .ok {
    color: var(--modrinth);
    font-size: 0.84rem;
    margin: 0 0 12px;
  }
</style>
