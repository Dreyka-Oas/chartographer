<script lang="ts">
  /** Section « Synchronisation » : relevé manuel et dernier passage par source. */
  import { formatAge } from "../../format";
  import { dashboard } from "../../state.svelte";
  import SettingRow from "./SettingRow.svelte";

  const freshness = $derived(dashboard.overview?.freshness ?? []);
</script>

<section id="synchronisation" class="stg-panel">
  <h2>Synchronisation</h2>
  <SettingRow
    name="Relevé automatique"
    desc="Au démarrage puis toutes les six heures. C'est ce rythme qui entretient les snapshots quotidiens CurseForge, seule source d'historique de cette plateforme."
  >
    {#snippet control()}
      <span class="stg-value">{formatAge(dashboard.dataAgeMs)}</span>
      <button class="stg-btn" onclick={() => dashboard.sync()} disabled={dashboard.syncing}>
        {dashboard.syncing ? "En cours…" : "Synchroniser"}
      </button>
    {/snippet}
  </SettingRow>
  {#if freshness.length > 0}
    <SettingRow name="Dernier passage par source">
      {#snippet control()}
        {#each freshness as entry (entry.provider)}
          <span class="chip" class:ko={entry.status !== "ok"} title={entry.detail}>
            {entry.provider}
            <b>{entry.finished_at ? entry.finished_at.slice(11, 16) : "jamais"}</b>
          </span>
        {/each}
      {/snippet}
    </SettingRow>
  {/if}
</section>

<style>
  /*
   * Le nom de la source et son heure n'ont pas la même fonte : sans hauteur de
   * ligne commune ni alignement des lignes de base, l'heure retombait sous le
   * texte.
   */
  .chip {
    font-size: 0.72rem;
    line-height: 1.6;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 10px;
    color: var(--text-dim);
    display: inline-flex;
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
</style>
