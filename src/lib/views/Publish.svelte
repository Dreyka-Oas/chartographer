<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { api } from "../api";
  import { BRANDS, type PlatformName } from "../components/brands";
  import Card from "../components/Card.svelte";
  import Select from "../components/Select.svelte";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, CfGesture, ProjectSummary, PublishOutcome } from "../types";

  const overview = $derived(dashboard.overview);
  const projects = $derived(overview?.per_project ?? []);

  let selectedKey = $state<string | null>(null);
  const project = $derived<ProjectSummary | null>(
    projects.find((p) => p.key === selectedKey) ?? null,
  );

  let filePath = $state("");
  let versionNumber = $state("");
  let displayName = $state("");
  let changelog = $state("");
  let releaseType = $state("release");
  let gameVersions = $state("");
  let loaders = $state("");
  let onModrinth = $state(true);
  let onCurseforge = $state(true);
  let manualRelease = $state(true);
  let running = $state(false);
  let outcomes = $state<PublishOutcome[]>([]);
  let removing = $state<string | null>(null);

  const fileName = $derived(filePath ? filePath.replace(/^.*[\\/]/, "") : "");

  /** Versions de jeu et chargeurs déjà rencontrés sur le compte : ils évitent
   * d'avoir à retaper ce que les fichiers publiés annoncent déjà. */
  const knownVersions = $derived(
    [...new Set((overview?.loaders ?? []).map((c) => c.game_version))]
      .filter(Boolean)
      .sort()
      .reverse()
      .slice(0, 24),
  );
  const knownLoaders = $derived(
    [...new Set((overview?.loaders ?? []).map((c) => c.loader))].filter(Boolean).sort(),
  );

  function report(e: unknown) {
    dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
  }

  function toggleIn(current: string, value: string): string {
    const parts = current
      .split(",")
      .map((p) => p.trim())
      .filter(Boolean);
    const at = parts.indexOf(value);
    if (at >= 0) parts.splice(at, 1);
    else parts.push(value);
    return parts.join(", ");
  }

  function has(current: string, value: string): boolean {
    return current
      .split(",")
      .map((p) => p.trim())
      .includes(value);
  }

  function split(raw: string): string[] {
    return raw
      .split(",")
      .map((p) => p.trim())
      .filter(Boolean);
  }

  // Le fichier arrive par glisser-déposer : rien à taper, rien à parcourir.
  let listening = $state(false);
  $effect(() => {
    if (listening) return;
    listening = true;
    let stop: (() => void) | null = null;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "drop" && event.payload.paths.length > 0) {
          filePath = event.payload.paths[0];
          if (!displayName) displayName = fileName.replace(/\.[^.]+$/, "");
        }
      })
      .then((unlisten) => (stop = unlisten))
      .catch(report);
    return () => stop?.();
  });

  const ready = $derived(
    filePath !== "" &&
      versionNumber.trim() !== "" &&
      project !== null &&
      (onModrinth || onCurseforge),
  );

  /**
   * Ce qui manque encore, dit en clair. Un bouton grisé sans explication laisse
   * chercher : l'envoi demande quatre choses, et rien à l'écran ne disait
   * laquelle faisait défaut.
   */
  const missing = $derived.by(() => {
    const manque: string[] = [];
    if (project === null) manque.push("un mod");
    if (filePath === "") manque.push("le fichier à envoyer");
    if (versionNumber.trim() === "") manque.push("un numéro de version");
    if (!onModrinth && !onCurseforge) manque.push("au moins une plateforme");
    if (manque.length === 0) return "";
    const dernier = manque.pop();
    return manque.length === 0
      ? `Il manque ${dernier}.`
      : `Il manque ${manque.join(", ")} et ${dernier}.`;
  });

  async function publish() {
    if (!project || !ready) return;
    running = true;
    outcomes = [];
    try {
      const sent = await api.publishVersion(
        {
          modrinth_project_id: onModrinth ? (project.modrinth_ext_id ?? null) : null,
          curseforge_project_id: onCurseforge ? (project.curseforge_ext_id ?? null) : null,
          name: displayName.trim() || versionNumber.trim(),
          version_number: versionNumber.trim(),
          changelog,
          game_versions: split(gameVersions),
          loaders: split(loaders),
          release_type: releaseType,
          manual_release: manualRelease,
        },
        filePath,
      );
      outcomes = sent.outcomes;
      await dashboard.sync();
    } catch (e) {
      report(e);
    } finally {
      running = false;
    }
  }

  // --- Gestes CurseForge -------------------------------------------------
  // Son interface publique d'envoi ne sait que déposer un fichier. Son tableau
  // de bord, lui, crée et retire — sans rien documenter, et un corps deviné
  // n'obtient qu'une erreur serveur muette. L'application regarde donc le geste
  // une fois, puis sait le refaire.
  let gestures = $state<CfGesture[]>([]);
  let watching = $state(false);
  let gestureNote = $state("");
  let newProjectName = $state("");
  let newProjectSummary = $state("");
  let creating = $state(false);
  let gesturesLoaded = $state(false);

  $effect(() => {
    if (gesturesLoaded) return;
    gesturesLoaded = true;
    api
      .curseforgeGestures()
      .then((value) => (gestures = value))
      .catch(report);
  });

  const knowsCreation = $derived(
    gestures.some((g) => g.method === "POST" && g.pattern.endsWith("/_api/projects")),
  );
  const knowsRemoval = $derived(
    gestures.some((g) => g.pattern.includes("file") && g.method !== "GET"),
  );

  async function watch() {
    try {
      await api.watchCurseforge();
      watching = true;
      gestureNote =
        "La fenêtre CurseForge est ouverte. Fais le geste une fois — créer un projet, retirer un fichier — puis reviens ici.";
    } catch (e) {
      report(e);
    }
  }

  async function learn() {
    try {
      gestures = await api.learnCurseforge();
      watching = false;
      gestureNote =
        gestures.length > 0
          ? `${gestures.length} geste(s) retenu(s).`
          : "Rien de neuf : le geste n'a peut-être pas abouti, ou la fenêtre a été rechargée.";
    } catch (e) {
      report(e);
    }
  }

  async function createProject() {
    if (!newProjectName.trim()) return;
    creating = true;
    try {
      const done = await api.createCurseforgeProject(
        newProjectName.trim(),
        newProjectSummary.trim(),
      );
      outcomes = [done, ...outcomes];
      if (done.ok) {
        newProjectName = "";
        newProjectSummary = "";
        await dashboard.sync();
      }
    } catch (e) {
      report(e);
    } finally {
      creating = false;
    }
  }

  /** Retire une version déposée par erreur. CurseForge ne le permet pas : son
   * interface d'envoi ne sait que déposer. */
  async function undo(outcome: PublishOutcome) {
    if (!outcome.id || outcome.platform !== "modrinth") return;
    removing = outcome.id;
    try {
      const done = await api.deleteModrinthVersion(outcome.id);
      outcomes = outcomes.map((o) => (o.id === outcome.id ? done : o));
      await dashboard.sync();
    } catch (e) {
      report(e);
    } finally {
      removing = null;
    }
  }
</script>

<div class="grid">
  <div class="wide">
    <Card
      title="Publier une version"
      subtitle="Un même fichier part sur les deux plateformes, avec les mêmes informations"
    >
      <div class="form">
        <div class="field">
          <span class="legend-label">Mod</span>
          <Select
            bind:value={selectedKey}
            label="Mod à publier"
            options={[
              { value: null, label: "choisir un mod…" },
              ...projects.map((entry) => ({ value: entry.key, label: entry.title })),
            ]}
          />
        </div>

        <div class="field">
          <span class="legend-label">Fichier</span>
          <div class="drop" class:filled={filePath !== ""}>
            {#if filePath}
              <b>{fileName}</b>
              <button class="link" onclick={() => (filePath = "")}>retirer</button>
            {:else}
              Dépose ici l'archive à publier : .jar, .zip, .mrpack ou .litemod.
            {/if}
          </div>
        </div>

        <label class="field">
          <span class="legend-label">Numéro de version</span>
          <input bind:value={versionNumber} placeholder="1.2.0" />
        </label>

        <label class="field">
          <span class="legend-label">Nom affiché</span>
          <input bind:value={displayName} placeholder="repris du numéro si vide" />
        </label>

        <div class="field">
          <span class="legend-label">Type</span>
          <Select
            bind:value={releaseType}
            label="Type de version"
            options={[
              { value: "release", label: "Version stable" },
              { value: "beta", label: "Bêta" },
              { value: "alpha", label: "Alpha" },
            ]}
          />
        </div>

        <label class="field wide-field">
          <span class="legend-label">Journal des changements</span>
          <textarea bind:value={changelog} rows="4" placeholder="Ce que cette version apporte."
          ></textarea>
        </label>

        <div class="field wide-field">
          <span class="legend-label">Versions du jeu</span>
          <input bind:value={gameVersions} placeholder="1.21.1, 1.21" />
          {#if knownVersions.length > 0}
            <div class="chips">
              {#each knownVersions as version (version)}
                <button
                  class="chip"
                  class:on={has(gameVersions, version)}
                  onclick={() => (gameVersions = toggleIn(gameVersions, version))}
                >
                  {version}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="field wide-field">
          <span class="legend-label">Chargeurs</span>
          <input bind:value={loaders} placeholder="fabric, neoforge" />
          {#if knownLoaders.length > 0}
            <div class="chips">
              {#each knownLoaders as loader (loader)}
                <button
                  class="chip"
                  class:on={has(loaders, loader)}
                  onclick={() => (loaders = toggleIn(loaders, loader))}
                >
                  {loader}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <div class="field wide-field targets">
          <label class="check">
            <input type="checkbox" bind:checked={onModrinth} />
            Modrinth
            {#if project && !project.modrinth_ext_id}<span class="warn">— mod absent</span>{/if}
          </label>
          <label class="check">
            <input type="checkbox" bind:checked={onCurseforge} />
            CurseForge
            {#if project && !project.curseforge_ext_id}<span class="warn">— mod absent</span>{/if}
          </label>
          <label class="check">
            <input type="checkbox" bind:checked={manualRelease} />
            Retenir le fichier CurseForge au lieu de le publier tout de suite
          </label>
        </div>
      </div>

      <div class="actions">
        <button class="primary" onclick={publish} disabled={!ready || running}>
          {running ? "Envoi en cours…" : "Publier"}
        </button>
        <span class="hint">
          {#if missing}
            {missing}
          {:else}
            Une version Modrinth se supprime d'ici. Côté CurseForge, le retrait passe par un geste
            appris — voir la carte ci-dessous.
          {/if}
        </span>
      </div>

      {#if outcomes.length > 0}
        <div class="results">
          {#each outcomes as outcome (outcome.platform + (outcome.id ?? ""))}
            <div class="result" class:ko={!outcome.ok}>
              <!-- Le nom de la plateforme est écrit comme elle l'écrit
                   elle-même : `outcome.platform` porte la clé interne, et
                   « modrinth » en gras minuscules détonnait au milieu d'une
                   page qui dit « Modrinth » partout ailleurs. -->
              <b>{BRANDS[outcome.platform as PlatformName]?.label ?? outcome.platform}</b>
              <span>{outcome.detail}</span>
              {#if outcome.ok && outcome.id && outcome.platform === "modrinth"}
                <button class="link" onclick={() => undo(outcome)} disabled={removing === outcome.id}>
                  {removing === outcome.id ? "suppression…" : "supprimer cette version"}
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </Card>
  </div>

  <div class="wide">
    <Card
      title="CurseForge — créer et retirer"
      subtitle="Ce que son interface d'envoi ne sait pas faire, son tableau de bord le fait"
    >
      <p class="hint block">
        CurseForge publie de quoi déposer un fichier, rien d'autre : ni créer un projet, ni retirer
        quoi que ce soit. Son tableau de bord, lui, fait les deux, par une voie qu'il ne documente
        pas et dont les champs ne se devinent pas. Alors montre-lui le geste une fois :
        l'application le regarde passer, en retient la forme exacte, et sait ensuite le refaire
        seule.
      </p>

      <div class="actions">
        <button onclick={watch} disabled={watching}>
          {watching ? "En observation…" : "Montrer un geste"}
        </button>
        <button class:primary={watching} onclick={learn}>Retenir ce que j'ai fait</button>
        <span class="hint">{gestureNote}</span>
      </div>

      {#if gestures.length > 0}
        <table>
          <thead>
            <tr><th class="left">Geste retenu</th><th class="left">Adresse</th></tr>
          </thead>
          <tbody>
            {#each gestures as gesture (gesture.method + gesture.pattern)}
              <tr>
                <td class="left">{gesture.method}</td>
                <td class="left mono">{gesture.pattern}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}

      <div class="form create">
        <label class="field">
          <span class="legend-label">Nom du nouveau projet</span>
          <input bind:value={newProjectName} placeholder="Mon nouveau mod" />
        </label>
        <label class="field">
          <span class="legend-label">Résumé</span>
          <input bind:value={newProjectSummary} placeholder="Ce que fait le mod, en une ligne" />
        </label>
        <div class="field">
          <span class="legend-label">&nbsp;</span>
          <button
            class="primary"
            onclick={createProject}
            disabled={!knowsCreation || creating || !newProjectName.trim()}
          >
            {creating ? "Création…" : "Créer sur CurseForge"}
          </button>
        </div>
      </div>
      {#if !knowsCreation}
        <p class="hint block">
          Le geste de création n'est pas encore connu : ouvre l'observation, crée un projet comme
          d'habitude, puis reviens le retenir.
        </p>
      {/if}
      {#if !knowsRemoval}
        <p class="hint block">
          Le retrait d'un fichier n'est pas encore connu non plus. Même méthode : supprime un
          fichier depuis le tableau de bord pendant l'observation.
        </p>
      {/if}
    </Card>
  </div>
</div>

<style>
  .block {
    display: block;
    margin: 0 0 14px;
    line-height: 1.55;
  }
  .create {
    margin-top: 16px;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }
  th {
    text-align: right;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--text-dim);
    font-weight: 500;
    font-size: 0.8rem;
  }
  td {
    text-align: right;
    padding: 5px 8px;
    border-bottom: 1px solid var(--border);
    font-size: 0.82rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 6px;
  }
  .left {
    text-align: left;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 16px;
  }
  .wide {
    grid-column: 1 / -1;
  }
  .form {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 14px 18px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .wide-field {
    grid-column: 1 / -1;
  }
  input,
  textarea {
    background-color: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    font-size: 0.86rem;
    padding: 7px 9px;
    width: 100%;
  }
  textarea {
    resize: vertical;
  }
  /*
   * La zone de dépôt occupe toute la hauteur de sa rangée, comme les champs qui
   * l'entourent. Laissée à sa hauteur propre, son texte sur deux lignes la
   * faisait descendre plus bas que ses voisins : la rangée s'alignait par le
   * haut, et elle seule dépassait.
   */
  .field .drop {
    flex: 1;
    min-height: 34px;
    border: 1px dashed var(--border);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    color: var(--text-dim);
    font-size: 0.8rem;
    line-height: 1.35;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .field .drop.filled {
    border-style: solid;
    color: var(--text);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }
  .chip {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text-dim);
    font: inherit;
    font-size: 0.76rem;
    padding: 3px 10px;
    cursor: pointer;
  }
  .chip.on {
    border-color: var(--accent);
    color: var(--text);
  }
  .targets {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.85rem;
  }
  .check input {
    width: auto;
  }
  .warn {
    color: var(--warn);
    font-size: 0.78rem;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    margin-top: 16px;
  }
  button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    font: inherit;
    font-size: 0.85rem;
    padding: 8px 14px;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
  .link {
    background: none;
    border: 0;
    color: var(--accent);
    padding: 0;
    font-size: 0.8rem;
    text-decoration: underline;
  }
  .hint {
    color: var(--text-dim);
    font-size: 0.78rem;
    max-width: 70ch;
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 16px;
  }
  .result {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border-left: 2px solid var(--modrinth);
    background: var(--surface-2);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    font-size: 0.84rem;
  }
  .result.ko {
    border-left-color: var(--error);
  }
</style>
