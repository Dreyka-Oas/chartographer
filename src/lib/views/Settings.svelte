<script lang="ts">
  /**
   * Page des réglages : elle ne dessine rien elle-même. Elle tient l'état —
   * ce qui est enregistré, ce qui vient d'être modifié — et distribue le reste
   * aux panneaux. Chaque section vit dans son propre fichier, et la mise en
   * page d'une rangée est décidée une seule fois, dans `SettingRow`.
   *
   * Rien ne se valide à la main : un réglage changé s'écrit tout seul, une
   * demi-seconde après la dernière frappe. Ce délai n'est pas du confort — il
   * évite d'écrire un fichier à chaque caractère d'un champ numérique.
   */
  import { api } from "../api";
  import { dashboard } from "../state.svelte";
  import type { AppErrorPayload, Settings } from "../types";
  import AccountPanel from "./settings/AccountPanel.svelte";
  import CursePanel from "./settings/CursePanel.svelte";
  import DisplayPanel from "./settings/DisplayPanel.svelte";
  import SettingsAside from "./settings/SettingsAside.svelte";
  import StatusHint from "./settings/StatusHint.svelte";
  import SyncPanel from "./settings/SyncPanel.svelte";
  import UpdatePanel from "./settings/UpdatePanel.svelte";

  /** Les valeurs que cette page modifie. Le reste ne fait que s'afficher. */
  type Editable = Pick<
    Settings,
    "range_days" | "currency" | "auto_sync_minutes" | "auto_update"
  >;

  const BLANK: Settings = {
    curseforge_username: null,
    range_days: 30,
    currency: "USD",
    auto_sync_minutes: 10,
    curseforge_token_ready: false,
    auto_update: true,
  };
  let saved = $state<Settings>({ ...BLANK });
  let draft = $state<Settings>({ ...BLANK });
  let hint = $state("");
  let tone = $state<"plain" | "done" | "error">("plain");
  let loaded = $state(false);

  /**
   * Dernier état écrit sur le disque, hors du système réactif à dessein : il
   * sert de point de comparaison à l'effet ci-dessous, et le relire ne doit pas
   * relancer celui-ci.
   */
  let written: Editable = { ...BLANK };
  let pending: ReturnType<typeof setTimeout> | null = null;
  let fading: ReturnType<typeof setTimeout> | null = null;

  function report(e: unknown) {
    dashboard.error = (e as AppErrorPayload)?.message ?? String(e);
  }

  /** Affiche un mot au bas de la page, qui s'efface seul. */
  function say(text: string, kind: "plain" | "done" | "error" = "plain", delay = 2000) {
    hint = text;
    tone = kind;
    if (fading !== null) clearTimeout(fading);
    fading = delay > 0 ? setTimeout(() => (hint = ""), delay) : null;
  }

  $effect(() => {
    if (loaded) return;
    loaded = true;
    api
      .getSettings()
      .then((value) => {
        saved = value;
        draft = { ...value };
        written = pick(value);
      })
      .catch(report);
  });

  function pick(value: Editable): Editable {
    return {
      range_days: value.range_days,
      currency: value.currency,
      auto_sync_minutes: value.auto_sync_minutes,
      auto_update: value.auto_update,
    };
  }

  /**
   * Un champ numérique vidé rend `null`, et une valeur hors bornes serait
   * ramenée dans celles-ci par le backend — le champ afficherait alors autre
   * chose que ce qui a été enregistré. On attend donc une valeur tenable.
   */
  function usable(value: Editable): boolean {
    const days = Number(value.range_days);
    const minutes = Number(value.auto_sync_minutes);
    return (
      Number.isInteger(days) &&
      days >= 7 &&
      days <= 730 &&
      Number.isInteger(minutes) &&
      minutes >= 10 &&
      minutes <= 1440 &&
      typeof value.currency === "string" &&
      value.currency.length === 3 &&
      typeof value.auto_update === "boolean"
    );
  }

  const same = (a: Editable, b: Editable) =>
    a.range_days === b.range_days &&
    a.currency === b.currency &&
    a.auto_sync_minutes === b.auto_sync_minutes &&
    a.auto_update === b.auto_update;

  /** Écrit le brouillon dès qu'il s'écarte de ce qui est sur le disque. */
  $effect(() => {
    const next = pick(draft);
    if (!loaded || same(next, written) || !usable(next)) return;
    if (pending !== null) clearTimeout(pending);
    pending = setTimeout(() => {
      pending = null;
      void commit(next);
    }, 500);
  });

  async function commit(next: Editable) {
    const changedCurrency = next.currency !== written.currency;
    const changedCadence = next.auto_sync_minutes !== written.auto_sync_minutes;
    say("Enregistrement…", "plain", 0);
    try {
      // Le pseudo CurseForge n'est plus saisi : il se relève tout seul. On
      // repasse celui qui est enregistré pour ne pas l'effacer.
      await api.saveSettings(
        saved.curseforge_username,
        next.range_days,
        next.currency,
        next.auto_sync_minutes,
        next.auto_update,
      );
      written = next;
      saved = { ...saved, ...next };
      // La cadence change tout de suite : attendre le prochain réveil, réglé
      // sur l'ancienne valeur, contredirait ce qui vient d'être enregistré.
      if (changedCadence) dashboard.restartAutoSync(next.auto_sync_minutes);
      // Changer de devise ne veut rien dire sans son taux : on le relève dans
      // la foulée, puis on redessine les montants déjà à l'écran.
      if (changedCurrency) await dashboard.refreshCurrency();
      say("Enregistré", "done");
    } catch (e) {
      report(e);
      say("Enregistrement impossible", "error", 4000);
    }
  }

  /** Retour du relevé de jeton CurseForge, mené par son panneau. */
  function tokenReady(ready: boolean) {
    saved = { ...saved, curseforge_token_ready: ready };
    draft = { ...draft, curseforge_token_ready: ready };
    say(
      ready
        ? "Jeton d'envoi relevé."
        : "Aucun jeton lisible : reconnecte-toi à CurseForge puis réessaie.",
      ready ? "done" : "error",
      4000,
    );
  }
</script>

<div class="layout">
  <SettingsAside />
  <div class="panels">
    <AccountPanel />
    <SyncPanel />
    <CursePanel ready={saved.curseforge_token_ready} onready={tokenReady} />
    <DisplayPanel {draft} />
    <UpdatePanel {draft} />
  </div>
</div>

<StatusHint text={hint} {tone} />

<style>
  /*
   * Seule la colonne des panneaux défile : le sommaire et le titre restent en
   * place, comme la barre de navigation au-dessus.
   */
  .layout {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
    gap: 28px;
    align-items: stretch;
    height: 100%;
    min-height: 0;
  }
  .panels {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
    overflow-y: auto;
    /*
     * Place de la barre de défilement réservée en permanence. Sans cela, la
     * colonne s'élargit et se rétrécit selon que la barre est là ou non, et
     * tout ce qu'elle contient bouge avec elle.
     */
    scrollbar-gutter: stable;
    overscroll-behavior: contain;
    padding-right: 4px;
    /* Le mot d'état flotte au-dessus du bas de page. */
    padding-bottom: 64px;
  }
</style>
