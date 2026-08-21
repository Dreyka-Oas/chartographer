<script lang="ts">
  import { formatAge } from "../format";
  import Tooltip from "./Tooltip.svelte";
  import type { Freshness } from "../types";

  let { entries }: { entries: Freshness[] } = $props();

  const failed = $derived(entries.filter((entry) => entry.status !== "ok"));

  /** Le relevé le plus ancien : c'est lui qui date l'ensemble. */
  const oldest = $derived(
    entries.reduce<number | null>((acc, entry) => {
      if (!entry.finished_at) return acc;
      const time = Date.parse(entry.finished_at);
      if (!Number.isFinite(time)) return acc;
      return acc === null || time < acc ? time : acc;
    }, null),
  );

  const age = $derived(oldest === null ? "jamais" : formatAge(Date.now() - oldest));

  /**
   * Quatre pastilles répétaient la même minute pour dire la même chose. Le
   * bandeau tient en une : l'âge du plus ancien relevé, et le nom des seules
   * sources qui ont échoué. Le détail complet reste au survol.
   */
  const detail = $derived(
    entries
      .map(
        (entry) =>
          `${entry.provider} : ${
            entry.finished_at ? entry.finished_at.slice(0, 16).replace("T", " ") : "jamais"
          }${entry.status === "ok" ? "" : `, ${entry.detail || entry.status}`}`,
      )
      .join("\n"),
  );
</script>

{#if entries.length > 0}
  <Tooltip text={detail} placement="bottom">
    <span class:ok={failed.length === 0} class:ko={failed.length > 0}>
      {#if failed.length === 0}
        Relevé {age}
      {:else}
        {failed.map((entry) => entry.provider).join(", ")} en échec
      {/if}
    </span>
  </Tooltip>
{/if}

<style>
  span {
    font-size: 0.72rem;
    border-radius: 999px;
    padding: 2px 9px;
    border: 1px solid var(--border);
    color: var(--text-dim);
    white-space: nowrap;
    cursor: default;
  }
  .ok {
    border-color: var(--modrinth);
    color: var(--modrinth);
  }
  .ko {
    border-color: var(--error);
    color: var(--error);
  }
</style>
