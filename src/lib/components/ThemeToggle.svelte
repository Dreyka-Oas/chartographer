<script lang="ts">
  import { theme } from "../theme.svelte";

  const icon = $derived(theme.mode === "auto" ? "◐" : theme.mode === "dark" ? "☾" : "☀");
  const hint = $derived(
    theme.mode === "auto"
      ? `Automatique (actuellement ${theme.dark ? "sombre" : "clair"})`
      : `Forcé en ${theme.mode === "dark" ? "sombre" : "clair"}`,
  );
</script>

<button onclick={() => theme.cycle()} title={hint} aria-label={hint}>
  <span aria-hidden="true">{icon}</span>
  {theme.label}
</button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 7px;
    padding: 4px 10px;
    font: inherit;
    font-size: 0.78rem;
    cursor: pointer;
  }
  button:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  span {
    font-size: 0.9rem;
    line-height: 1;
  }
</style>
