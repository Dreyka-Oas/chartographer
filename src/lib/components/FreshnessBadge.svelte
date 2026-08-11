<script lang="ts">
  import type { Freshness } from "../types";

  let { entries }: { entries: Freshness[] } = $props();
</script>

<div>
  {#each entries as entry (entry.provider)}
    <span class:ok={entry.status === "ok"} class:ko={entry.status !== "ok"} title={entry.detail}>
      {entry.provider} · {entry.finished_at
        ? entry.finished_at.slice(0, 16).replace("T", " ")
        : "jamais"}
    </span>
  {/each}
</div>

<style>
  div {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  span {
    font-size: 0.72rem;
    border-radius: 999px;
    padding: 2px 9px;
    border: 1px solid var(--border);
    color: var(--text-dim);
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
