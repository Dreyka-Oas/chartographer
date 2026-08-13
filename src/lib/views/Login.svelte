<script lang="ts">
  import { api } from "../api";
  import ThemeToggle from "../components/ThemeToggle.svelte";
  import { dashboard } from "../state.svelte";

  let token = $state("");

  /**
   * Autorisations du token Modrinth, sous les intitulés exacts de la page de
   * création. Chacune répond à un appel précis de l'application ; rien n'est
   * demandé au-delà.
   */
  const READ_SCOPES = [
    { label: "Read user data", why: "reconnaître le compte" },
    { label: "Read projects", why: "lister les projets" },
    { label: "Read versions", why: "suivre les fichiers publiés" },
    { label: "Read analytics", why: "téléchargements et revenus par jour" },
    { label: "Read payouts", why: "solde en attente" },
    { label: "Read notifications", why: "les avis reçus" },
  ];

  /** À cocher en plus pour publier depuis l'application, jamais pour la seule lecture. */
  const WRITE_SCOPES = [
    { label: "Create versions", why: "envoyer un fichier" },
    { label: "Create projects", why: "ouvrir un projet" },
    { label: "Delete versions", why: "retirer un fichier" },
    { label: "Delete projects", why: "supprimer un projet" },
  ];

  let publishing = $state(false);

  const modrinthOn = $derived(dashboard.auth?.connected === true);
  const curseforgeOn = $derived(dashboard.curseforgeSession === true);
  /** Ce qui reste à faire, dit en une phrase plutôt qu'en deux pastilles. */
  const missing = $derived.by(() => {
    if (!modrinthOn && !curseforgeOn) return "Aucun des deux comptes n'est relié.";
    if (!modrinthOn) return "CurseForge est relié. Il manque Modrinth.";
    if (!curseforgeOn) return "Modrinth est relié. Il manque CurseForge.";
    return "";
  });
</script>

<div class="screen">
  <h1>Chartographer</h1>
  <p class="tagline">Tes statistiques Modrinth et CurseForge sur un seul écran.</p>

  <!--
    L'état des deux comptes est montré d'emblée : l'application ne s'ouvre
    qu'avec les deux, autant dire tout de suite lequel manque.
  -->
  <div class="state">
    <span class="badge" class:on={modrinthOn}>
      <span class="dot modrinth"></span>
      Modrinth {modrinthOn ? "relié" : "à relier"}
    </span>
    <span class="badge" class:on={curseforgeOn}>
      <span class="dot curseforge"></span>
      CurseForge {curseforgeOn ? "relié" : "à relier"}
    </span>
  </div>
  {#if missing}<p class="missing">{missing}</p>{/if}

  {#if !modrinthOn}
    <ol class="steps">
      <li>
        <button class="link" onclick={() => api.openTokenPage()}>
          Ouvrir mes tokens Modrinth
        </button>
        puis clique <b>Create a PAT</b>.
      </li>
      <li>
        Coche ces six autorisations, toutes en lecture :
        <span class="scopes">
          {#each READ_SCOPES as scope (scope.label)}
            <code title={scope.why}>{scope.label}</code>
          {/each}
        </span>
      </li>
      <li>
        <label class="more">
          <input type="checkbox" bind:checked={publishing} />
          Je veux aussi publier mes fichiers depuis l'application.
        </label>
        {#if publishing}
          <span class="scopes">
            {#each WRITE_SCOPES as scope (scope.label)}
              <code title={scope.why}>{scope.label}</code>
            {/each}
          </span>
          <span class="aside">
            Sans elles, tout le reste fonctionne : seul l'onglet Publier refusera l'envoi.
          </span>
        {/if}
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
  {/if}

  {#if dashboard.error}<p class="error">{dashboard.error}</p>{/if}

  {#if !curseforgeOn}
    <!--
      CurseForge n'a pas de token à créer à la main : ses chiffres se lisent
      dans une fenêtre de navigateur, avec la session du compte, et le jeton
      d'envoi est demandé puis relevé par l'application le jour où tu publies.
    -->
    <div class="curseforge">
      <span class="cf-title">CurseForge</span>
      <p>
        Aucune clé à créer. Connecte-toi à ton compte dans la fenêtre que l'application ouvre :
        elle y lit ensuite le tableau de bord auteur, et crée toute seule le jeton d'envoi, sous
        le nom « Chartographer », le jour où tu publies un fichier.
      </p>
      <div class="cf-actions">
        <button class="primary" onclick={() => api.openCurseforgeWindow()}>
          Ouvrir la fenêtre CurseForge
        </button>
        <button
          class="link"
          disabled={dashboard.checkingCurseforge}
          onclick={() => dashboard.checkCurseforge()}
        >
          {dashboard.checkingCurseforge ? "Vérification…" : "J'ai fini, vérifier"}
        </button>
      </div>
    </div>
  {/if}

  <p class="hint">
    Les deux comptes sont demandés avant d'entrer : l'application met leurs chiffres côte à côte,
    et un total amputé de moitié ne se verrait pas. Le token Modrinth reste sur cette machine et
    n'est jamais transmis ailleurs qu'à Modrinth.
  </p>

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
    margin: 0 0 4px;
    color: var(--text-dim);
  }
  .state {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 0.78rem;
    color: var(--text-dim);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 4px 12px;
  }
  .badge.on {
    color: var(--text);
    border-color: var(--accent);
  }
  /* La pastille garde la couleur de sa plateforme, et pâlit tant qu'elle manque. */
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    opacity: 0.3;
  }
  .badge.on .dot {
    opacity: 1;
  }
  .dot.modrinth {
    background: var(--modrinth);
  }
  .dot.curseforge {
    background: var(--curseforge);
  }
  .missing {
    margin: 0 0 4px;
    font-size: 0.82rem;
    color: var(--text-dim);
  }
  .steps {
    margin: 0;
    padding: 0 0 0 20px;
    max-width: 56ch;
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
    margin: 5px 0 2px;
  }
  .more {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .more input {
    accent-color: var(--accent);
    margin: 0;
  }
  .aside {
    display: block;
    font-size: 0.78rem;
    line-height: 1.5;
    opacity: 0.85;
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
  form input {
    flex: 1;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text);
    padding: 10px 12px;
    font: inherit;
  }
  form input:focus {
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
  .link:disabled {
    opacity: 0.5;
    cursor: default;
  }
  /* Le second compte est présenté à part : il ne se relie pas de la même façon,
   * et rien n'y est à préparer avant de commencer. */
  .curseforge {
    max-width: 56ch;
    text-align: left;
    border-top: 1px solid var(--border);
    padding-top: 14px;
    margin-top: 4px;
  }
  .cf-title {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--curseforge);
  }
  .curseforge p {
    margin: 4px 0 10px;
    color: var(--text-dim);
    font-size: 0.82rem;
    line-height: 1.6;
  }
  .cf-actions {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.78rem;
    max-width: 56ch;
    margin: 0;
  }
  .error {
    color: var(--error);
    font-size: 0.84rem;
    max-width: 52ch;
  }
</style>
