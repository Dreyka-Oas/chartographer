use crate::config::{Session, Settings};
use crate::error::{AppError, Result};
use crate::matching::{match_projects, Candidate};
use crate::models::Platform;
use crate::providers::curseforge::{CfFetch, CurseForgeClient};
use crate::providers::modrinth::{timestamp_to_day, ModrinthClient};
use crate::store::metrics as m;
use crate::store::projects as p;
use crate::store::queries::shift_day;
use crate::store::Store;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;
use serde::Serialize;

/// Fenêtre maximale récupérée lors du tout premier rafraîchissement.
pub const INITIAL_WINDOW_DAYS: i64 = 365;

#[derive(Debug, Clone, Serialize)]
pub struct SyncReport {
    pub provider: String,
    pub status: String,
    pub detail: String,
}

/// Jalon d'une étape du cycle, posé à son ouverture puis à sa clôture.
#[derive(Debug, Clone, Serialize)]
pub struct SyncStep {
    pub provider: String,
    /// Compte rendu de l'étape, nul tant qu'elle est en cours.
    pub report: Option<SyncReport>,
}

impl SyncStep {
    fn opening(provider: &str) -> Self {
        Self {
            provider: provider.to_string(),
            report: None,
        }
    }

    fn closing(report: &SyncReport) -> Self {
        Self {
            provider: report.provider.clone(),
            report: Some(report.clone()),
        }
    }
}

/// Tout ce dont la synchronisation a besoin : la session OAuth et les réglages.
pub struct SyncContext {
    pub session: Session,
    pub settings: Settings,
}

pub fn today_utc() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

pub fn window_start(last_known_day: Option<&str>, today: &str, fallback_days: i64) -> String {
    match last_known_day {
        Some(day) => day.to_string(),
        None => shift_day(today, -fallback_days),
    }
}

pub fn username_candidates(modrinth_username: &str) -> Vec<String> {
    vec![
        modrinth_username.to_string(),
        format!("{modrinth_username}_official"),
    ]
}

fn to_datetime(day: &str) -> DateTime<Utc> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|d| Utc.from_local_datetime(&d).single())
        .unwrap_or_else(Utc::now)
}

pub fn apply_matches(conn: &Connection) -> Result<usize> {
    let projects = p::list(conn)?;
    let to_candidate = |platform: Platform| -> Vec<Candidate> {
        projects
            .iter()
            .filter(|x| x.platform == platform && x.archived_at.is_none())
            .map(|x| Candidate {
                id: x.id,
                slug: x.slug.clone(),
                title: x.title.clone(),
            })
            .collect()
    };

    p::clear_automatic_links(conn)?;
    let matches = match_projects(
        &to_candidate(Platform::Modrinth),
        &to_candidate(Platform::CurseForge),
    );
    for found in &matches {
        p::upsert_link(
            conn,
            found.modrinth_id,
            found.curseforge_id,
            found.confidence,
            false,
        )?;
    }
    Ok(matches.len())
}

async fn run_provider<F>(store: &Store, provider: &str, work: F) -> SyncReport
where
    F: std::future::Future<Output = Result<String>>,
{
    let started = Utc::now().to_rfc3339();
    let run_id = store
        .with(|conn| m::start_sync_run(conn, provider, &started))
        .unwrap_or(0);

    let (status, detail) = match work.await {
        Ok(detail) => ("ok".to_string(), detail),
        Err(e) => ("failed".to_string(), e.to_string()),
    };

    let finished = Utc::now().to_rfc3339();
    let _ = store.with(|conn| m::finish_sync_run(conn, run_id, &finished, &status, &detail));
    SyncReport {
        provider: provider.to_string(),
        status,
        detail,
    }
}

async fn discover_modrinth(store: &Store, ctx: &SyncContext) -> Result<String> {
    let client = ModrinthClient::new(&ctx.session.token)?;
    let user = client.me().await?;
    let projects = client.projects(&user.id).await?;
    let now = Utc::now().to_rfc3339();
    let seen: Vec<String> = projects.iter().map(|x| x.id.clone()).collect();

    store.with(|conn| {
        for project in &projects {
            p::upsert(
                conn,
                &p::ProjectUpsert {
                    platform: Platform::Modrinth,
                    ext_id: project.id.clone(),
                    slug: Some(project.slug.clone()),
                    title: project.title.clone(),
                    project_type: project.project_type.clone(),
                    url: Some(format!("https://modrinth.com/mod/{}", project.slug)),
                    icon_url: project.icon_url.clone(),
                    created_at: project.published.clone(),
                    total_downloads: project.downloads,
                    followers: Some(project.followers),
                },
            )?;
        }
        p::archive_missing(conn, Platform::Modrinth, &seen, &now)?;
        m::set_meta(
            conn,
            "modrinth_balance",
            &user.payout_data.balance.to_string(),
        )?;
        Ok(())
    })?;

    // Le solde détaillé vit sur une route séparée. Son absence ne doit pas
    // faire échouer la découverte : les projets, eux, sont déjà enregistrés.
    let payout = match client.payout_balance().await {
        Ok(balance) => {
            let raw = serde_json::to_string(&balance)?;
            store.with(|conn| m::set_meta(conn, "modrinth_payout", &raw))?;
            format!(", {} $ retirables", balance.available)
        }
        Err(_) => String::new(),
    };

    Ok(format!("{} projets{payout}", projects.len()))
}

/// Déduit le pseudo auteur CurseForge sans rien demander.
/// D'abord en interrogeant CFWidget avec les slugs Modrinth déjà connus, ce qui
/// donne le pseudo réel quel qu'il soit ; à défaut, en essayant les variantes
/// dérivées du pseudo Modrinth.
async fn resolve_curseforge_author(
    client: &CurseForgeClient,
    store: &Store,
    ctx: &SyncContext,
) -> Result<crate::providers::curseforge::CfAuthor> {
    if let Some(name) = ctx.settings.curseforge_username.as_deref() {
        return client.author(name).await;
    }

    let slugs: Vec<String> = store
        .with(|conn| p::list_by_platform(conn, Platform::Modrinth))?
        .into_iter()
        .filter_map(|row| row.slug)
        .collect();

    for slug in &slugs {
        if let Some(owner) = client.owner_of_slug(slug).await {
            if let Ok(found) = client.author(&owner).await {
                return Ok(found);
            }
        }
    }

    let fallbacks = username_candidates(&ctx.session.username);
    for candidate in &fallbacks {
        if candidate.is_empty() {
            continue;
        }
        if let Ok(found) = client.author(candidate).await {
            return Ok(found);
        }
    }

    Err(AppError::Config(format!(
        "auteur CurseForge introuvable : ni via les {} slugs Modrinth, ni parmi {}",
        slugs.len(),
        fallbacks.join(", ")
    )))
}

async fn discover_curseforge(store: &Store, ctx: &SyncContext) -> Result<String> {
    let client = CurseForgeClient::new()?;
    let author = resolve_curseforge_author(&client, store, ctx).await?;
    // L'auteur retenu est conservé : l'interface l'affiche, et il évite de
    // resonder les slugs au cycle suivant.
    store.with(|conn| m::set_meta(conn, "curseforge_author", &author.username))?;

    let now = Utc::now().to_rfc3339();
    let mut seen: Vec<String> = Vec::new();
    let mut queued = 0usize;

    for entry in &author.projects {
        match client.project(entry.id).await? {
            CfFetch::Queued => queued += 1,
            CfFetch::Ready(project) => {
                seen.push(project.id.to_string());
                store.with(|conn| {
                    let project_id = p::upsert(
                        conn,
                        &p::ProjectUpsert {
                            platform: Platform::CurseForge,
                            ext_id: project.id.to_string(),
                            slug: project.slug.clone(),
                            title: project.title.clone(),
                            project_type: project.project_type.clone(),
                            url: project.url.clone(),
                            icon_url: project.thumbnail.clone(),
                            created_at: project.created_at.clone(),
                            total_downloads: project.downloads_total,
                            followers: None,
                        },
                    )?;
                    m::insert_snapshot(
                        conn,
                        project_id,
                        &now,
                        project.downloads_total,
                        Some(project.downloads_monthly),
                    )?;

                    // Les fichiers publiés sont le seul historique que CurseForge
                    // expose : datés, avec leur version de jeu et leur chargeur.
                    for file in &project.files {
                        m::upsert_version(
                            conn,
                            project_id,
                            &file.id.to_string(),
                            Some(&file.display),
                            &file.game_versions,
                            &file.loaders,
                            file.downloads,
                            file.uploaded_at.as_deref(),
                        )?;
                    }
                    Ok(())
                })?;
            }
        }
    }

    if !seen.is_empty() {
        store.with(|conn| p::archive_missing(conn, Platform::CurseForge, &seen, &now))?;
    }
    store.with(|conn| m::set_meta(conn, "curseforge_username", &author.username))?;

    Ok(format!(
        "{} projets, {queued} en file d'attente",
        seen.len()
    ))
}

async fn refresh_modrinth(store: &Store, ctx: &SyncContext) -> Result<String> {
    let client = ModrinthClient::new(&ctx.session.token)?;
    let user_id = ctx.session.user_id.clone();

    let rows = store.with(|conn| p::list_by_platform(conn, Platform::Modrinth))?;
    let ids: Vec<String> = rows.iter().map(|r| r.ext_id.clone()).collect();
    if ids.is_empty() {
        return Ok("aucun projet".into());
    }
    let by_ext: std::collections::HashMap<String, i64> =
        rows.iter().map(|r| (r.ext_id.clone(), r.id)).collect();

    let today = today_utc();
    let last = store.with(m::last_metrics_day)?;
    let start = to_datetime(&window_start(last.as_deref(), &today, INITIAL_WINDOW_DAYS));
    let end = to_datetime(&shift_day(&today, 1));

    let downloads = client.analytics_downloads(&ids, start, end).await?;
    let views = client.analytics_views(&ids, start, end).await?;
    let revenue = client.analytics_revenue(&ids, start, end).await?;
    let countries = client.analytics_countries(&ids, start, end).await?;

    store.with(|conn| {
        for (ext_id, series) in &downloads {
            let Some(project_id) = by_ext.get(ext_id) else {
                continue;
            };
            for (ts, value) in series {
                m::upsert_daily(
                    conn,
                    *project_id,
                    &timestamp_to_day(*ts),
                    Some(*value),
                    None,
                    None,
                )?;
            }
        }
        for (ext_id, series) in &views {
            let Some(project_id) = by_ext.get(ext_id) else {
                continue;
            };
            for (ts, value) in series {
                m::upsert_daily(
                    conn,
                    *project_id,
                    &timestamp_to_day(*ts),
                    None,
                    Some(*value),
                    None,
                )?;
            }
        }
        for (ext_id, series) in &revenue {
            let Some(project_id) = by_ext.get(ext_id) else {
                continue;
            };
            for (ts, value) in series {
                m::upsert_daily(
                    conn,
                    *project_id,
                    &timestamp_to_day(*ts),
                    None,
                    None,
                    Some(&value.to_string()),
                )?;
            }
        }
        for (ext_id, per_country) in &countries {
            let Some(project_id) = by_ext.get(ext_id) else {
                continue;
            };
            for (code, value) in per_country {
                m::upsert_country(conn, *project_id, &today, code, *value)?;
            }
        }
        Ok(())
    })?;

    for row in &rows {
        let versions = client.versions(&row.ext_id).await?;
        store.with(|conn| {
            for version in &versions {
                m::upsert_version(
                    conn,
                    row.id,
                    &version.id,
                    version.version_number.as_deref(),
                    &version.game_versions,
                    &version.loaders,
                    version.downloads,
                    version.date_published.as_deref(),
                )?;
            }
            Ok(())
        })?;
    }

    let notifications = client.notifications(&user_id).await?;
    store.with(|conn| {
        for notification in &notifications {
            let project_id = notification
                .project_ext_id
                .as_ref()
                .and_then(|ext| by_ext.get(ext).copied());
            let title = project_id
                .and_then(|id| rows.iter().find(|r| r.id == id))
                .map(|r| r.title.clone())
                .unwrap_or_else(|| "Modrinth".into());
            m::insert_event(
                conn,
                "modrinth",
                &notification.occurred_at,
                &notification.kind,
                project_id,
                &title,
                &notification.detail,
            )?;
        }
        Ok(())
    })?;

    Ok(format!(
        "{} projets, {} notifications",
        rows.len(),
        notifications.len()
    ))
}

async fn snapshot_curseforge(store: &Store) -> Result<String> {
    let client = CurseForgeClient::new()?;
    let rows = store.with(|conn| p::list_by_platform(conn, Platform::CurseForge))?;
    let now = Utc::now().to_rfc3339();
    let mut written = 0usize;
    let mut queued = 0usize;

    for row in &rows {
        let Ok(id) = row.ext_id.parse::<i64>() else {
            continue;
        };
        match client.project(id).await? {
            CfFetch::Queued => queued += 1,
            CfFetch::Ready(project) => {
                store.with(|conn| {
                    m::insert_snapshot(
                        conn,
                        row.id,
                        &now,
                        project.downloads_total,
                        Some(project.downloads_monthly),
                    )?;
                    p::upsert(
                        conn,
                        &p::ProjectUpsert {
                            platform: Platform::CurseForge,
                            ext_id: row.ext_id.clone(),
                            slug: project.slug.clone(),
                            title: project.title.clone(),
                            project_type: project.project_type.clone(),
                            url: project.url.clone(),
                            icon_url: project.thumbnail.clone(),
                            created_at: project.created_at.clone(),
                            total_downloads: project.downloads_total,
                            followers: None,
                        },
                    )?;
                    Ok(())
                })?;
                written += 1;
            }
        }
    }
    Ok(format!("{written} snapshots, {queued} en file d'attente"))
}

/// Cycle complet : découverte, appariement, rafraîchissement, snapshot.
/// Aucun échec ne bloque les autres providers.
pub async fn full(store: &Store, ctx: &SyncContext) -> Vec<SyncReport> {
    full_with_progress(store, ctx, |_| {}).await
}

/// Le même cycle, chaque étape annoncée à son ouverture puis à sa clôture.
///
/// L'écran de démarrage joue toute la synchronisation avant d'ouvrir la page.
/// Sans ces jalons, il n'aurait qu'une longue attente muette à montrer, alors
/// que le cycle enchaîne cinq travaux bien distincts.
pub async fn full_with_progress<F>(store: &Store, ctx: &SyncContext, on: F) -> Vec<SyncReport>
where
    F: Fn(SyncStep) + Send + Sync,
{
    let mut reports = Vec::new();
    reports.push(staged(store, "modrinth", discover_modrinth(store, ctx), &on).await);
    reports.push(staged(store, "curseforge", discover_curseforge(store, ctx), &on).await);

    on(SyncStep::opening("matching"));
    let matched = store.with(apply_matches);
    let matching = match &matched {
        Ok(count) => SyncReport {
            provider: "matching".into(),
            status: "ok".into(),
            detail: format!("{count} liens automatiques"),
        },
        Err(e) => SyncReport {
            provider: "matching".into(),
            status: "failed".into(),
            detail: e.to_string(),
        },
    };
    on(SyncStep::closing(&matching));
    // L'appariement ne figure au compte rendu que s'il a échoué : réussi, il
    // n'apprend rien de plus que les liens qu'il vient d'écrire.
    if matched.is_err() {
        reports.push(matching);
    }

    reports.push(staged(store, "modrinth-analytics", refresh_modrinth(store, ctx), &on).await);
    reports.push(staged(store, "curseforge-snapshot", snapshot_curseforge(store), &on).await);
    reports
}

/// Un provider joué entre ses deux jalons.
async fn staged<F, W>(store: &Store, provider: &str, work: W, on: &F) -> SyncReport
where
    F: Fn(SyncStep) + Send + Sync,
    W: std::future::Future<Output = Result<String>>,
{
    on(SyncStep::opening(provider));
    let report = run_provider(store, provider, work).await;
    on(SyncStep::closing(&report));
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::projects::{links, list};
    use crate::store::Store;

    #[test]
    fn analytics_window_starts_after_the_last_known_day() {
        assert_eq!(
            window_start(Some("2026-08-05"), "2026-08-11", 365),
            "2026-08-05"
        );
        assert_eq!(window_start(None, "2026-08-11", 30), "2026-07-12");
    }

    #[test]
    fn candidate_username_variants_are_ordered() {
        assert_eq!(
            username_candidates("DreykaOas"),
            vec!["DreykaOas".to_string(), "DreykaOas_official".to_string()]
        );
    }

    #[test]
    fn apply_matches_writes_links_for_identical_titles() {
        let store = Store::open_in_memory().unwrap();
        store
            .with(|conn| {
                use crate::store::projects::{upsert, ProjectUpsert};
                let mk = |platform, ext: &str, slug: &str, title: &str| ProjectUpsert {
                    platform,
                    ext_id: ext.into(),
                    slug: Some(slug.into()),
                    title: title.into(),
                    project_type: None,
                    url: None,
                    icon_url: None,
                    created_at: None,
                    total_downloads: 0,
                    followers: None,
                };
                upsert(
                    conn,
                    &mk(Platform::Modrinth, "m1", "mobsblocker", "Mobs Blocker"),
                )
                .unwrap();
                upsert(
                    conn,
                    &mk(Platform::CurseForge, "c1", "mobblocker", "Mobs Blocker"),
                )
                .unwrap();
                apply_matches(conn)
            })
            .unwrap();

        let rows = store.with(links).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].confidence, 1.0);
        assert_eq!(store.with(list).unwrap().len(), 2);
    }
}
