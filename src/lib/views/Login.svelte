<script lang="ts">
  import { api } from "../api";
  import ThemeToggle from "../components/ThemeToggle.svelte";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload } from "../types";

  let clientId = $state("");
  let clientSecret = $state("");
  let saving = $state(false);

  const needsApp = $derived(dashboard.auth?.oauth_app_configured === false);

  async function saveApp() {
    saving = true;
    try {
      await api.saveOauthApp(clientId, clientSecret);
      await dashboard.refreshAuth();
    } catch (e) {
      dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="screen">
  <h1>Chartographer</h1>
  <p class="tagline">Tes statistiques Modrinth et CurseForge sur un seul écran.</p>

  {#if needsApp}
    <div class="setup">
      <p>
        Enregistre une application OAuth sur <code>modrinth.com/settings/applications</code>
        avec <code>http://127.0.0.1/callback</code> comme URL de redirection, puis colle ses
        identifiants ici. Cette étape disparaît si l'application est compilée avec
        <code>MODRINTH_CLIENT_ID</code> et <code>MODRINTH_CLIENT_SECRET</code>.
      </p>
      <input bind:value={clientId} placeholder="client_id" />
      <input bind:value={clientSecret} type="password" placeholder="client_secret" />
      <button onclick={saveApp} disabled={saving || !clientId || !clientSecret}>Enregistrer</button>
    </div>
  {:else}
    <button class="primary" onclick={() => dashboard.login()} disabled={dashboard.connecting}>
      {dashboard.connecting ? "En attente du navigateur…" : "Se connecter avec Modrinth"}
    </button>
    <p class="hint">
      Ton navigateur va s'ouvrir sur la page d'autorisation Modrinth. Rien à copier, rien à coller.
    </p>
  {/if}

  {#if dashboard.error}<p class="error">{dashboard.error}</p>{/if}

  <div class="corner"><ThemeToggle /></div>
</div>

<style>
  .corner {
    position: fixed;
    top: 14px;
    right: 16px;
  }
  .screen {
    min-height: 100vh;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 10px;
    padding: 24px;
    text-align: center;
  }
  h1 {
    margin: 0;
    font-size: 2rem;
    font-weight: 600;
  }
  .tagline {
    margin: 0 0 18px;
    color: var(--text-dim);
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border: 0;
    border-radius: 9px;
    padding: 12px 26px;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .primary:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.82rem;
    max-width: 36ch;
  }
  .setup {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 48ch;
  }
  .setup p {
    color: var(--text-dim);
    font-size: 0.84rem;
    line-height: 1.5;
    text-align: left;
  }
  code {
    background: var(--surface-2);
    border-radius: 4px;
    padding: 1px 5px;
    font-size: 0.9em;
  }
  input {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 7px;
    color: var(--text);
    padding: 9px 11px;
    font: inherit;
  }
  button {
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: 7px;
    padding: 9px 14px;
    font: inherit;
    cursor: pointer;
  }
  .error {
    color: var(--error);
    font-size: 0.84rem;
  }
</style>
