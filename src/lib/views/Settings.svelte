<script lang="ts">
  import { api } from "../api";
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
</script>

<h1>Réglages</h1>

<section>
  <h2>Compte Modrinth</h2>
  {#if dashboard.auth?.connected}
    <p>
      Connecté en tant que <b>{dashboard.auth.username}</b> depuis
      {dashboard.auth.connected_since?.slice(0, 10)}.
    </p>
    <button onclick={() => dashboard.logout()}>Se déconnecter</button>
  {:else}
    <button onclick={() => dashboard.login()}>Se connecter avec Modrinth</button>
  {/if}
</section>

<section>
  <h2>CurseForge</h2>
  <p class="hint">
    Détecté automatiquement depuis ton pseudo Modrinth. Renseigne-le seulement si la détection
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
  <input type="number" min="7" max="730" bind:value={settings.range_days} />
  <span class="hint">jours affichés par défaut</span>
</section>

<section>
  <h2>Appariements manquants</h2>
  {#if modrinthOrphans.length === 0 && curseforgeOrphans.length === 0}
    <p class="hint">Tous les projets sont appariés.</p>
  {:else}
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

<button class="primary" onclick={save}>Enregistrer</button>
{#if message}<p class="ok">{message}</p>{/if}

<style>
  h1 {
    font-size: 1.4rem;
    margin: 0 0 18px;
  }
  h2 {
    font-size: 0.95rem;
    margin: 0 0 8px;
  }
  section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
    margin-bottom: 12px;
    max-width: 720px;
  }
  p {
    margin: 0 0 10px;
    font-size: 0.86rem;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.8rem;
  }
  input,
  select {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text);
    padding: 8px 10px;
    font: inherit;
    font-size: 0.86rem;
  }
  button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 7px;
    padding: 8px 14px;
    font: inherit;
    font-size: 0.86rem;
    cursor: pointer;
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
  }
</style>
