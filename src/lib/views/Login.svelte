<script lang="ts">
  import { api } from "../api";
  import ThemeToggle from "../components/ThemeToggle.svelte";
  import { dashboard } from "../state.svelte";

  let token = $state("");

  // Portées à cocher sur la page de création du token. Tout est en lecture seule.
  const SCOPES = [
    "Read user data",
    "Read notifications",
    "Read payouts",
    "Access analytics",
    "Read projects",
    "Read versions",
  ];
</script>

<div class="screen">
  <h1>Chartographer</h1>
  <p class="tagline">Tes statistiques Modrinth et CurseForge sur un seul écran.</p>

  <ol class="steps">
    <li>
      <button class="link" onclick={() => api.openTokenPage()}>
        Ouvrir mes tokens Modrinth
      </button>
      puis clique <b>Create a PAT</b>.
    </li>
    <li>
      Coche ces six autorisations, toutes en lecture seule :
      <span class="scopes">
        {#each SCOPES as scope (scope)}<code>{scope}</code>{/each}
      </span>
    </li>
    <li>Copie le token et colle-le ici.</li>
  </ol>

  <form
    onsubmit={(e) => {
      e.preventDefault();
      dashboard.connect(token);
    }}
  >
    <input
      bind:value={token}
      type="password"
      placeholder="mrp_…"
      autocomplete="off"
      spellcheck="false"
    />
    <button class="primary" type="submit" disabled={dashboard.connecting || !token.trim()}>
      {dashboard.connecting ? "Vérification…" : "Se connecter"}
    </button>
  </form>

  <p class="hint">
    Le token reste sur cette machine et n'est jamais transmis ailleurs qu'à Modrinth. CurseForge ne
    demande rien : tes projets y sont retrouvés automatiquement.
  </p>

  {#if dashboard.error}<p class="error">{dashboard.error}</p>{/if}

  <div class="corner"><ThemeToggle /></div>
</div>

<style>
  .screen {
    min-height: 100vh;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 12px;
    padding: 24px;
    text-align: center;
  }
  .corner {
    position: fixed;
    top: 14px;
    right: 16px;
  }
  h1 {
    margin: 0;
    font-size: 2rem;
    font-weight: 600;
  }
  .tagline {
    margin: 0 0 10px;
    color: var(--text-dim);
  }
  .steps {
    margin: 0;
    padding: 0 0 0 20px;
    max-width: 52ch;
    text-align: left;
    color: var(--text-dim);
    font-size: 0.86rem;
    line-height: 1.7;
  }
  .steps b {
    color: var(--text);
  }
  .scopes {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    margin-top: 5px;
  }
  code {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 1px 6px;
    font-size: 0.78rem;
    color: var(--text);
  }
  form {
    display: flex;
    gap: 8px;
    width: min(460px, 100%);
    margin-top: 6px;
  }
  input {
    flex: 1;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    padding: 10px 12px;
    font: inherit;
  }
  input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border: 0;
    border-radius: 8px;
    padding: 10px 20px;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .link {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: var(--accent);
    text-decoration: underline;
    cursor: pointer;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.78rem;
    max-width: 52ch;
    margin: 0;
  }
  .error {
    color: var(--error);
    font-size: 0.84rem;
    max-width: 52ch;
  }
</style>
