<script lang="ts">
  import Select from "./Select.svelte";
  import { formatMonth, formatRange, lastDayOfMonth } from "../format";
  import { dashboard } from "../state.svelte";

  const RANGES = [30, 90, 180, 365];

  const overview = $derived(dashboard.overview);
  const months = $derived([...(overview?.available_months ?? [])].reverse());
  /** Fenêtre glissante : aucune borne explicite n'a été posée. */
  const sliding = $derived(dashboard.rangeFrom === null && dashboard.rangeTo === null);
  /** Mois calendaire exactement couvert par la fenêtre, s'il y en a un. */
  const activeMonth = $derived(
    dashboard.rangeFrom !== null &&
      dashboard.rangeTo !== null &&
      dashboard.rangeFrom.endsWith("-01") &&
      dashboard.rangeTo === lastDayOfMonth(dashboard.rangeFrom.slice(0, 7))
      ? dashboard.rangeFrom.slice(0, 7)
      : "",
  );

  let from = $state("");
  let to = $state("");

  // Les champs suivent la fenêtre effective renvoyée par le backend : changer de
  // préréglage ou de mois les repositionne sans que l'utilisateur les ressaisisse.
  $effect(() => {
    if (!overview) return;
    from = overview.from;
    to = overview.to;
  });

  function pickMonth(value: string) {
    if (value === "") {
      dashboard.setRange(dashboard.rangeDays);
    } else {
      dashboard.setMonth(value);
    }
  }
</script>

<div class="picker">
  <div class="group" role="group" aria-label="Fenêtre glissante">
    {#each RANGES as days (days)}
      <button
        class:active={sliding && dashboard.rangeDays === days}
        onclick={() => dashboard.setRange(days)}
      >
        {days} j
      </button>
    {/each}
  </div>

  <div class="month">
    <span class="legend-label">Mois</span>
    <div class="list">
      <Select
        value={activeMonth}
        label="Mois relevé"
        compact
        onchange={(value) => pickMonth(value ?? "")}
        options={[
          { value: "", label: "Tous" },
          ...months.map((month) => ({ value: month, label: formatMonth(month) })),
        ]}
      />
    </div>
  </div>

  <label class="days">
    <span class="legend-label">Du</span>
    <input type="date" bind:value={from} max={to} />
    <span class="legend-label">au</span>
    <input type="date" bind:value={to} min={from} />
    <button
      class="apply"
      disabled={!from || !to || (from === overview?.from && to === overview?.to)}
      onclick={() => dashboard.setCustomRange(from, to)}
    >
      Appliquer
    </button>
  </label>

  {#if overview}
    <span class="window" title="Fenêtre affichée">{formatRange(overview.from, overview.to)}</span>
  {/if}
</div>

<style>
  .picker {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .group {
    display: flex;
    gap: 4px;
  }
  label,
  .month {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  /* La liste des mois garde une largeur constante : les intitulés varient. */
  .list {
    width: 148px;
  }
  button,
  input {
    background-color: var(--surface);
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: var(--radius-sm);
    padding: 5px 10px;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
    transition:
      color 120ms ease,
      border-color 120ms ease;
  }
  input {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  button.active,
  button:hover:not(:disabled),
  input:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .window {
    font-size: 0.78rem;
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
</style>
