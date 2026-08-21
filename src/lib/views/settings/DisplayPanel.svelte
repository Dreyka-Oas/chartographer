<script lang="ts">
  /** Section "Affichage" : fenêtre, cadence, devise et thème. */
  import Select from "../../components/Select.svelte";
  import { formatDayLong } from "../../format";
  import { dashboard } from "../../state.svelte";
  import { theme, type ThemeMode } from "../../theme.svelte";
  import type { Settings } from "../../types";
  import SettingRow from "./SettingRow.svelte";

  /** Le brouillon est modifié sur place : le parent le tient, et compare. */
  let { draft }: { draft: Settings } = $props();

  /** Devises proposées. Les deux plateformes paient en dollars ; les autres
   * passent par le taux de référence relevé chaque jour. */
  const CURRENCIES = [
    { value: "USD", label: "Dollar américain ($)" },
    { value: "EUR", label: "Euro (€)" },
    { value: "GBP", label: "Livre sterling (£)" },
    { value: "CHF", label: "Franc suisse (CHF)" },
    { value: "CAD", label: "Dollar canadien ($ CA)" },
    { value: "JPY", label: "Yen (¥)" },
  ];

  const THEMES: { mode: ThemeMode; label: string }[] = [
    { mode: "auto", label: "Automatique" },
    { mode: "light", label: "Clair" },
    { mode: "dark", label: "Sombre" },
  ];

  /**
   * Le taux arrive après le premier rendu. Il occupe sa propre ligne, dont la
   * place est réservée d'avance : la description ne grandit donc pas sous le
   * contrôle une fois le relevé arrivé.
   */
  const rate = $derived.by(() => {
    const currency = dashboard.overview?.currency;
    if (!currency?.day) return "";
    const value = currency.rate.toFixed(4).replace(".", ",");
    return `Dernier taux : 1 $ = ${value} ${currency.code}, au ${formatDayLong(currency.day)}.`;
  });
</script>

<section id="affichage" class="stg-panel">
  <h2>Affichage</h2>
  <SettingRow
    name="Fenêtre par défaut"
    desc="Nombre de jours chargés à l'ouverture de la page de vision."
  >
    {#snippet control()}
      <input class="stg-input" type="number" min="7" max="730" bind:value={draft.range_days} />
      <span class="stg-unit">jours</span>
    {/snippet}
  </SettingRow>

  <SettingRow
    name="Cadence des relevés"
    desc="Délai entre deux relevés. L'attente réelle varie d'un quart autour de cette valeur : des relevés parfaitement réguliers se remarqueraient, et CurseForge ne se lit qu'à travers une session de navigateur. Dix minutes au plus court."
  >
    {#snippet control()}
      <input
        class="stg-input"
        type="number"
        min="10"
        max="1440"
        bind:value={draft.auto_sync_minutes}
      />
      <span class="stg-unit">minutes</span>
    {/snippet}
  </SettingRow>

  <SettingRow
    name="Devise"
    desc="Les deux plateformes paient en dollars. Choisir une autre monnaie convertit les montants au taux de référence de la Banque centrale européenne, relevé automatiquement."
    note={rate}
  >
    {#snippet control()}
      <!-- Largeur figée : le champ ne change pas de taille au gré du nom de la
           devise choisie, et il remplit la colonne, qui ne bouge pas non plus. -->
      <div class="picker">
        <Select
          bind:value={draft.currency}
          label="Devise d'affichage"
          align="end"
          options={CURRENCIES}
        />
      </div>
    {/snippet}
  </SettingRow>

  <SettingRow
    name="Thème"
    desc="En automatique, l'application suit le réglage clair ou sombre de Windows."
  >
    {#snippet control()}
      {#each THEMES as option (option.mode)}
        <button
          class="stg-btn"
          class:on={theme.mode === option.mode}
          onclick={() => theme.set(option.mode)}
        >
          {option.label}
        </button>
      {/each}
    {/snippet}
  </SettingRow>
</section>

<style>
  /* Assez large pour le plus long nom de devise, pas davantage : le champ ne
   * remplit pas sa colonne, il se cale à droite comme les autres contrôles. */
  .picker {
    width: 186px;
    flex: none;
  }
</style>
