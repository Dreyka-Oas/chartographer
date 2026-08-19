<script lang="ts">
  import Chart from "../charts/Chart.svelte";
  import { rankingOption, stackedProjectsOption } from "../charts/multiseries";
  import { palette } from "../charts/theme";
  import StatRow from "../components/StatRow.svelte";
  import { compactNumber, countryLabel, formatDayLong, formatMoney } from "../format";
  import { dashboard } from "../state.svelte";
  import { theme } from "../theme.svelte";
  import DetailShell from "./detail/DetailShell.svelte";

  const detail = $derived(dashboard.project);
  const summary = $derived(detail?.summary ?? null);

  const total = $derived(
    summary ? summary.modrinth_downloads + summary.curseforge_downloads : 0,
  );
  const share = $derived(
    !summary || total === 0 ? 0 : Math.round((summary.modrinth_downloads / total) * 100),
  );

  const periodDownloads = $derived(
    detail ? detail.downloads.reduce((s, v) => s + v, 0) + detail.curseforge.reduce((s, v) => s + v, 0) : 0,
  );
  const periodViews = $derived(detail ? detail.views.reduce((s, v) => s + v, 0) : 0);
  const periodRevenue = $derived(
    detail ? detail.revenue.reduce((s, v) => s + (Number.parseFloat(v) || 0), 0) : 0,
  );
  const conversion = $derived(
    periodViews === 0 ? null : Math.round((periodDownloads / periodViews) * 100),
  );

  const series = $derived(
    detail
      ? stackedProjectsOption(
          detail.days,
          [
            { name: "Modrinth", values: detail.downloads },
            { name: "CurseForge", values: detail.curseforge },
            { name: "Vues", values: detail.views },
          ].filter((s) => s.values.some((v) => v > 0)),
          palette(theme.dark),
          false,
        )
      : null,
  );

  const knownCountries = $derived(detail?.countries.filter((c) => c.country !== "??") ?? []);
  const countryChart = $derived(
    rankingOption(
      [...knownCountries].slice(0, 10).reverse().map((c) => countryLabel(c.country)),
      [...knownCountries].slice(0, 10).reverse().map((c) => c.downloads),
      palette(theme.dark),
    ),
  );

  const topVersions = $derived(
    detail ? [...detail.versions].sort((a, b) => b.downloads - a.downloads).slice(0, 20) : [],
  );
</script>

{#if dashboard.projectLoading}
  <DetailShell title="Chargement…"><p class="empty">Lecture des séries…</p></DetailShell>
{:else if detail && summary}
  <DetailShell
    title={summary.title}
    subtitle="{summary.modrinth_id !== null ? 'Modrinth' : ''}{summary.modrinth_id !== null &&
    summary.curseforge_id !== null
      ? ' et '
      : ''}{summary.curseforge_id !== null ? 'CurseForge' : ''} · {detail.days.length} jours"
    icon={summary.icon_url}
  >
    <StatRow
      stats={[
        {
          label: "Téléchargements",
          value: compactNumber(total),
          hint: `${compactNumber(periodDownloads)} sur la période`,
        },
        {
          label: "Équilibre plateformes",
          value: `${share} / ${100 - share}`,
          hint: "Modrinth / CurseForge",
        },
        {
          label: "Vues Modrinth",
          value: compactNumber(periodViews),
          hint: conversion === null ? "aucune vue" : `${conversion} % de conversion`,
        },
        {
          label: "Revenus sur la période",
          value: formatMoney(String(periodRevenue)),
          hint: `${summary.followers} followers`,
        },
      ]}
    />

    <div class="panel">
      <h2>Activité quotidienne</h2>
      {#if series}
        <Chart option={series} height={380} />
      {/if}
    </div>

    <div class="grid">
      <div class="panel">
        <h2>Origine des téléchargements</h2>
        {#if knownCountries.length === 0}
          <p class="empty">Aucune donnée géographique sur cette période.</p>
        {:else}
          <Chart option={countryChart} height={Math.max(200, knownCountries.slice(0, 10).length * 30 + 40)} />
        {/if}
      </div>

      <div class="panel">
        <h2>Appariement inter-plateformes</h2>
        {#if summary.link_confidence === null}
          <p class="note">
            Ce projet n'existe que sur une plateforme, ou son jumeau n'a pas été retrouvé
            automatiquement. Le rapprochement est retenté à chaque synchronisation.
          </p>
        {:else}
          <p class="big">{Math.round(summary.link_confidence * 100)} %</p>
          <p class="note">
            {summary.link_confidence === 1
              ? "Correspondance exacte sur le slug ou le titre."
              : "Correspondance approchée sur le titre. Vérifie qu'il s'agit bien du même mod."}
          </p>
        {/if}
        <dl>
          <div><dt>Modrinth</dt><dd>{compactNumber(summary.modrinth_downloads)}</dd></div>
          <div><dt>CurseForge</dt><dd>{compactNumber(summary.curseforge_downloads)}</dd></div>
          <div><dt>Followers</dt><dd>{summary.followers}</dd></div>
        </dl>
      </div>
    </div>

    <div class="panel wide">
      <h2>Versions publiées</h2>
      {#if topVersions.length === 0}
        <p class="empty">Aucune version indexée pour ce projet.</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th class="left">Version</th>
              <th class="left">Versions de jeu</th>
              <th class="left">Loaders</th>
              <th>Téléchargements</th>
              <th class="left">Publiée</th>
            </tr>
          </thead>
          <tbody>
            {#each topVersions as version (version.version_number ?? version.date_published)}
              <tr>
                <td class="left mono">{version.version_number ?? "—"}</td>
                <td class="left">
                  <span class="tags">
                    {#each version.game_versions.slice(0, 6) as gv (gv)}<code>{gv}</code>{/each}
                    {#if version.game_versions.length > 6}
                      <span class="more">+{version.game_versions.length - 6}</span>
                    {/if}
                  </span>
                </td>
                <td class="left">
                  <span class="tags">
                    {#each version.loaders as loader (loader)}<code>{loader}</code>{/each}
                  </span>
                </td>
                <td>{compactNumber(version.downloads)}</td>
                <td class="left dim">
                  {version.date_published ? formatDayLong(version.date_published.slice(0, 10)) : "—"}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  </DetailShell>
{/if}

<style>
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(380px, 1fr));
    gap: 14px;
    margin-top: 14px;
  }
  .wide {
    margin-top: 14px;
  }
  h2 {
    margin: 0 0 10px;
    font-family: var(--font-display);
    font-size: 0.98rem;
    font-weight: 600;
  }
  .big {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 1.6rem;
    font-weight: 600;
  }
  .note {
    margin: 6px 0 0;
    font-size: 0.8rem;
    color: var(--text-dim);
    max-width: 60ch;
  }
  .empty {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin: 0;
  }
  dl {
    margin: 14px 0 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    border-top: 1px solid var(--border);
    padding-top: 12px;
  }
  dt {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-dim);
  }
  dd {
    margin: 3px 0 0;
    font-family: var(--font-mono);
    font-size: 1.05rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.84rem;
  }
  th {
    text-align: right;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--text-dim);
    font-weight: 500;
  }
  td {
    text-align: right;
    padding: 7px 8px;
    border-bottom: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
    vertical-align: top;
  }
  .left {
    text-align: left;
  }
  .mono {
    font-family: var(--font-mono);
  }
  .dim {
    color: var(--text-dim);
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  code {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0 5px;
    font-family: var(--font-mono);
    font-size: 0.72rem;
  }
  .more {
    font-size: 0.72rem;
    color: var(--text-dim);
  }
  tbody tr:hover {
    background: var(--surface-2);
  }
</style>
