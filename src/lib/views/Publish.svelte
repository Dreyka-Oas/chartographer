<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { api } from "../api";
  import Card from "../components/Card.svelte";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, ProjectSummary, PublishOutcome } from "../types";

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

  async function publish() {
    if (!project || !ready) return;
    running = true;
    outcomes = [];
    try {
      const report = await api.publishVersion(
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
      outcomes = report.outcomes;
      await dashboard.sync();
    } catch (e) {
      report(e);
    } finally {
      running = false;
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
        <label class="field">
          <span class="legend-label">Mod</span>
          <select bind:value={selectedKey}>
            <option value={null}>choisir un mod…</option>
            {#each projects as entry (entry.key)}
              <option value={entry.key}>{entry.title}</option>
            {/each}
          </select>
        </label>

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

        <label class="field">
          <span class="legend-label">Type</span>
          <select bind:value={releaseType}>
            <option value="release">Version stable</option>
            <option value="beta">Bêta</option>
            <option value="alpha">Alpha</option>
          </select>
        </label>

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
          Modrinth accepte la suppression d'une version ; CurseForge, non : son interface d'envoi ne
          sait que déposer, le retrait se fait sur son site.
        </span>
      </div>

      {#if outcomes.length > 0}
        <div class="results">
          {#each outcomes as outcome (outcome.platform + (outcome.id ?? ""))}
            <div class="result" class:ko={!outcome.ok}>
              <b>{outcome.platform}</b>
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
</div>

<style>
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
  select,
  textarea {
    background: var(--surface-2);
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
  .drop {
    border: 1px dashed var(--border);
    border-radius: var(--radius-sm);
    padding: 14px;
    color: var(--text-dim);
    font-size: 0.84rem;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .drop.filled {
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
