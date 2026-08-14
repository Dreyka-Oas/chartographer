# Classement des journées — plan d'implémentation

> **Pour les agents :** SOUS-COMPÉTENCE REQUISE — utiliser `superpowers:subagent-driven-development` (recommandé) ou `superpowers:executing-plans` pour dérouler ce plan tâche par tâche. Les étapes sont cochables (`- [ ]`).

**But :** ajouter une page « Classement des journées » qui montre, sur la période choisie, un graphique et la liste des journées avec leur date, leurs téléchargements par plateforme, leurs revenus et leur rang — rang dans la période et rang rétrospectif à quatre-vingt-dix jours, celui déjà affiché sur la page Journée.

**Architecture :** un nouveau module Rust `store/rankings.rs` calcule les rangs à partir de `queries::timeline`, la seule source qui sait mêler mesures quotidiennes et écarts de snapshots CurseForge. Le calcul du rang rétrospectif, aujourd'hui écrit à même `day_report`, y est extrait pour servir aux deux (aucune duplication). Côté interface, la page est une vue plein écran bâtie sur `DetailShell`, `RankedTable`, `StatRow`, `Chart` et le composant `Hint` existants — aucun de ces composants n'est recopié.

**Pile :** Rust + Tauri 2 + rusqlite + rust_decimal côté données ; Svelte 5 (runes) + TypeScript + ECharts 6 côté interface. Tests : `cargo test` (Rust) et `vitest` (front).

**Spec :** la demande, telle que formulée — « une page qui affiche un graphique / la liste des journées avec le classement et les bonnes dates », avec le même « ? » que partout ailleurs (composant partagé, pas de code dupliqué), et une vérification réelle de ce que Modrinth et CurseForge fournissent avant de déclarer quoi que ce soit impossible.

## Ce que les deux plateformes fournissent réellement (vérifié le 2026-08-14)

Vérifié sur la base locale `%APPDATA%\fr.dreykaoas.chartographer\chartographer.db` (643 Ko) et sur le code des fournisseurs. Aucune de ces valeurs n'est supposée.

| Donnée | Modrinth | CurseForge |
| --- | --- | --- |
| Téléchargements par jour | oui, `GET /v3/analytics/downloads?...&resolution_minutes=1440` ([modrinth.rs:332-362](../../../src-tauri/src/providers/modrinth.rs#L332-L362)) | pas d'API publique de statistiques auteur ; l'application relève le tableau de bord auteur et complète par l'écart entre deux snapshots ([queries.rs:119-127](../../../src-tauri/src/store/queries.rs#L119-L127)) |
| Revenus par jour | oui, `GET /v3/analytics/revenue` | non ; reconstruits par écart de solde de points ([metrics.rs:76-103](../../../src-tauri/src/store/metrics.rs#L76-L103)) |
| Profondeur en base, aujourd'hui | 369 jours, du 2025-08-11 au 2026-08-14 | 110 jours, du 2024-03-19 au 2026-08-13 |
| Revenus en base | 1 827 lignes, du 2025-08-11 au 2026-08-13 | 4 relevés de points seulement (`cf_points`) |

Conséquences, à porter dans l'interface plutôt qu'à taire :

1. **Le total par jour n'a pas la même assise selon l'époque.** Avant le 2025-08-11, seul CurseForge est relevé. La page annonce donc la première date connue de chaque plateforme et le dit dans une bulle d'aide.
2. **Les revenus quotidiens sont pour ainsi dire ceux de Modrinth.** Les points CurseForge n'étant relevés qu'au passage, la colonne des revenus le précise dans sa bulle d'aide plutôt que d'afficher des zéros muets.
3. **Aucune requête réseau nouvelle n'est nécessaire :** tout est déjà en base. Le classement se calcule hors ligne.

## Contraintes globales

- Commentaires et libellés en français, avec accents. Ton du dépôt : on explique le pourquoi, jamais le comment.
- `Set-Content -Encoding utf8` corrompt l'UTF-8 sur ce poste : écrire les fichiers accentués avec les outils Write/Edit.
- Un fichier, une responsabilité ; viser moins de 150 lignes par fichier neuf.
- Le point d'interrogation d'aide est **toujours** `src/lib/components/Hint.svelte`. Ne jamais réécrire de bouton `?`, de bulle ou de placement.
- Les tableaux classés passent **toujours** par `src/lib/components/RankedTable.svelte`.
- La borne haute d'une plage est exclue partout côté Rust (`to` exclusif), et ramenée au dernier jour affiché avant d'être rendue à l'interface.
- Aucun `Date.now()` ni horodatage figé dans les tests.
- Les commandes se lancent depuis `C:\Users\ipmss\chartographer` en PowerShell (le Bash de ce poste est cassé).

---

## Structure des fichiers

**Créés**

- `src-tauri/src/store/rankings.rs` — rang d'une journée parmi ses voisines, revenus par jour, et la liste classée complète. Environ 130 lignes plus ses tests.
- `src/lib/charts/dayRanking.ts` — les deux options ECharts de la page : barres par jour et courbe du rang.
- `src/lib/charts/dayRanking.test.ts` — tests des deux fabriques.
- `src/lib/views/detail/DaysDetail.svelte` — la page.

**Modifiés**

- `src-tauri/src/store/mod.rs:1-5` — déclarer le module.
- `src-tauri/src/models.rs` — `DayRankRow` et `DayRankings`.
- `src-tauri/src/store/queries.rs:892-940` — `day_report` consomme `rankings::rank_within` et `rankings::revenue_by_day` au lieu de refaire les deux calculs.
- `src-tauri/src/commands.rs` — commande `day_rankings`.
- `src-tauri/src/lib.rs:81` — enregistrer la commande.
- `src/lib/types.ts` — les deux interfaces miroir.
- `src/lib/api.ts:60` — `dayRankings`.
- `src/lib/state.svelte.ts:18-25` — `DetailView` gagne `"days"`.
- `src/App.svelte:143-162` — brancher la vue.
- `src/lib/views/Day.svelte:82-121` — bouton d'accès depuis le rang.

---

## Tâche 1 : extraire le rang et les revenus par jour dans un module dédié

**Fichiers :**
- Créer : `src-tauri/src/store/rankings.rs`
- Modifier : `src-tauri/src/store/mod.rs:1-5`
- Modifier : `src-tauri/src/store/queries.rs:892-940`

**Interfaces :**
- Consomme : `queries::timeline`, `queries::shift_day`, `queries::PlatformFilter`, `metrics::cf_points_gained`, `models::Platform`.
- Produit :
  - `pub const RANK_WINDOW_DAYS: i64 = 90;`
  - `pub fn rank_within(window: &[i64], total: i64) -> Option<i64>`
  - `pub fn revenue_by_day(conn: &Connection, from: &str, to: &str, filter: PlatformFilter) -> Result<BTreeMap<String, (Decimal, Decimal)>>` — clé : le jour ; valeur : (Modrinth, CurseForge), en dollars.

- [ ] **Étape 1 : écrire les tests qui échouent**

Créer `src-tauri/src/store/rankings.rs` avec ce seul contenu pour l'instant :

```rust
//! Rang d'une journée parmi celles qui l'ont précédée.
//!
//! Le classement vit à part des autres requêtes parce qu'il répond à une autre
//! question : non pas « combien », mais « était-ce un bon jour quand il s'est
//! produit ». Il ne regarde donc jamais en avant.

use crate::error::Result;
use crate::models::Platform;
use crate::store::queries::{shift_day, PlatformFilter};
use rusqlite::{params, Connection};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

/// Longueur de la fenêtre de comparaison, en jours, la journée jugée comprise.
pub const RANK_WINDOW_DAYS: i64 = 90;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::metrics::upsert_daily;
    use crate::store::projects::{upsert, upsert_link, ProjectUpsert};
    use crate::store::schema::migrate;

    #[test]
    fn rank_counts_only_better_days() {
        assert_eq!(rank_within(&[10, 30, 20], 30), Some(1));
        assert_eq!(rank_within(&[10, 30, 20], 20), Some(2));
        assert_eq!(rank_within(&[10, 30, 20], 10), Some(3));
    }

    /// Une journée sans le moindre téléchargement n'est pas un jour creux :
    /// c'est un jour sans relevé, et il n'a pas de rang.
    #[test]
    fn a_day_without_downloads_has_no_rank() {
        assert_eq!(rank_within(&[10, 30], 0), None);
    }

    /// Deux journées identiques partagent le même rang : la seconde ne perd
    /// pas une place pour être arrivée après.
    #[test]
    fn equal_days_share_their_rank() {
        assert_eq!(rank_within(&[50, 50, 10], 50), Some(1));
    }
}
```

- [ ] **Étape 2 : lancer les tests et vérifier qu'ils échouent**

Lancer : `cd C:\Users\ipmss\chartographer\src-tauri; cargo test rankings`
Attendu : ÉCHEC de compilation, `cannot find function 'rank_within' in this scope` (et `unresolved module` tant que `mod.rs` n'est pas modifié).

- [ ] **Étape 3 : déclarer le module**

Dans `src-tauri/src/store/mod.rs`, ajouter la ligne dans l'ordre alphabétique existant :

```rust
pub mod followers;
pub mod metrics;
pub mod projects;
pub mod queries;
pub mod rankings;
pub mod schema;
```

- [ ] **Étape 4 : écrire `rank_within`**

Dans `rankings.rs`, avant le `mod tests` :

```rust
/// Rang de `total` parmi les totaux de sa fenêtre, 1 étant le meilleur.
///
/// Les journées à zéro sont écartées par l'appelant : ce sont des jours sans
/// relevé, pas des jours creux, et elles flatteraient le classement. Le rang
/// compte les journées strictement meilleures, si bien que deux journées
/// égales portent le même rang.
pub fn rank_within(window: &[i64], total: i64) -> Option<i64> {
    (total > 0).then(|| window.iter().filter(|other| **other > total).count() as i64 + 1)
}
```

- [ ] **Étape 5 : lancer les tests et vérifier qu'ils passent**

Lancer : `cd C:\Users\ipmss\chartographer\src-tauri; cargo test rankings`
Attendu : SUCCÈS, 3 tests.

- [ ] **Étape 6 : écrire le test des revenus par jour**

Ajouter dans `mod tests` de `rankings.rs` :

```rust
    /// Base de test : un mod des deux côtés, reliés, comme dans `queries`.
    fn seed() -> (Connection, i64, i64) {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let mk = |platform, ext: &str, title: &str, total| ProjectUpsert {
            platform,
            ext_id: ext.into(),
            slug: Some(title.to_lowercase().replace(' ', "-")),
            title: title.into(),
            project_type: Some("mod".into()),
            url: None,
            icon_url: None,
            created_at: None,
            total_downloads: total,
            followers: Some(5),
        };
        let m = upsert(&conn, &mk(Platform::Modrinth, "m1", "Mobs Blocker", 23_225)).unwrap();
        let c = upsert(
            &conn,
            &mk(Platform::CurseForge, "1002185", "Mobs Blocker", 86_753),
        )
        .unwrap();
        upsert_link(&conn, m, c, 1.0, false).unwrap();
        (conn, m, c)
    }

    #[test]
    fn revenue_is_returned_day_by_day() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, Some("1.25")).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(300), None, Some("3.50")).unwrap();

        let by_day = revenue_by_day(&conn, "2026-08-10", "2026-08-12", PlatformFilter::default())
            .unwrap();
        assert_eq!(by_day.get("2026-08-10").unwrap().0, Decimal::from_str("1.25").unwrap());
        assert_eq!(by_day.get("2026-08-11").unwrap().0, Decimal::from_str("3.50").unwrap());
        assert_eq!(by_day.len(), 2, "les jours sans revenu ne sont pas inventés");
    }

    /// Masquer Modrinth doit vider ses revenus, pas les laisser passer sous un
    /// autre nom.
    #[test]
    fn hiding_modrinth_hides_its_revenue() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, Some("1.25")).unwrap();
        let filter = PlatformFilter { modrinth: false, curseforge: true };
        let by_day = revenue_by_day(&conn, "2026-08-10", "2026-08-11", filter).unwrap();
        assert!(by_day.get("2026-08-10").map(|v| v.0).unwrap_or_default().is_zero());
    }
```

Ce `seed()` est celui de `src-tauri/src/store/queries.rs:1096-1119`, recopié tel quel : `upsert` prend un `&ProjectUpsert`, pas une liste de paramètres. Vérifier de même la signature d'`upsert_daily` dans `src-tauri/src/store/metrics.rs:140-150`.

- [ ] **Étape 7 : lancer les tests et vérifier qu'ils échouent**

Lancer : `cd C:\Users\ipmss\chartographer\src-tauri; cargo test rankings`
Attendu : ÉCHEC, `cannot find function 'revenue_by_day' in this scope`.

- [ ] **Étape 8 : écrire `revenue_by_day`**

```rust
/// Contre-valeur en dollars d'un point CurseForge, telle que la base la connaît.
fn point_value() -> Decimal {
    Decimal::from_str(&crate::store::metrics::CF_POINT_VALUE_USD.to_string()).unwrap_or_default()
}

/// Revenus jour par jour, plateforme par plateforme, `to` exclu.
///
/// Modrinth relève ses revenus quotidiennement. CurseForge n'en publie aucun :
/// ce qui apparaît ici est l'écart entre deux soldes de points, rapporté au
/// jour où il a été constaté. Les journées sans le moindre revenu sont absentes
/// de la table plutôt que portées à zéro : rien n'a été relevé, ce qui n'est
/// pas la même chose que rien gagné.
pub fn revenue_by_day(
    conn: &Connection,
    from: &str,
    to: &str,
    filter: PlatformFilter,
) -> Result<BTreeMap<String, (Decimal, Decimal)>> {
    let mut out: BTreeMap<String, (Decimal, Decimal)> = BTreeMap::new();

    if filter.modrinth {
        let mut stmt = conn.prepare(
            "SELECT m.day, m.revenue FROM metrics_daily m
             JOIN projects p ON p.id = m.project_id
             WHERE m.revenue IS NOT NULL AND m.day >= ?1 AND m.day < ?2
               AND p.platform = ?3",
        )?;
        let rows = stmt.query_map(params![from, to, Platform::Modrinth.as_str()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (day, raw) = row?;
            out.entry(day).or_default().0 += Decimal::from_str(&raw).unwrap_or_default();
        }
    }

    if filter.curseforge {
        let mut day = from.to_string();
        while day.as_str() < to {
            let next = shift_day(&day, 1);
            let gained = crate::store::metrics::cf_points_gained(conn, &day, &next)?;
            if gained != 0 {
                out.entry(day.clone()).or_default().1 += Decimal::from(gained) * point_value();
            }
            day = next;
        }
    }

    Ok(out)
}
```

- [ ] **Étape 9 : lancer les tests et vérifier qu'ils passent**

Lancer : `cd C:\Users\ipmss\chartographer\src-tauri; cargo test rankings`
Attendu : SUCCÈS, 5 tests.

- [ ] **Étape 10 : faire consommer ces deux fonctions par `day_report`**

Dans `src-tauri/src/store/queries.rs`, la closure `revenue_of` (lignes 892-914) et le calcul du rang (lignes 933-940) refont ce que `rankings` sait maintenant faire. Remplacer la closure par un appel, et le calcul du rang par `rank_within`.

Remplacer les lignes 892-914 (`let revenue_of = |from: &str, to: &str| ... };`) par :

```rust
    let revenue_of = |from: &str, to: &str| -> Result<(Decimal, Decimal)> {
        Ok(crate::store::rankings::revenue_by_day(conn, from, to, filter)?
            .values()
            .fold((Decimal::ZERO, Decimal::ZERO), |acc, (m, c)| (acc.0 + m, acc.1 + c)))
    };
```

Remplacer les lignes 933-940 (du `let window_start = ...` au `let rank = ...`) par :

```rust
    let window_start = shift_day(day, -(crate::store::rankings::RANK_WINDOW_DAYS - 1));
    let neighbours: Vec<i64> = timeline(conn, &window_start, &next, filter)?
        .iter()
        .map(|p| p.modrinth + p.curseforge)
        .filter(|total| *total > 0)
        .collect();
    let total = point.modrinth + point.curseforge;
    let rank = crate::store::rankings::rank_within(&neighbours, total);
```

Le commentaire qui précède (lignes 926-932) reste : il explique toujours ce que le code fait, et c'est le seul endroit où le lecteur de `day_report` le trouvera.

- [ ] **Étape 11 : vérifier que la page Journée n'a pas bougé d'un chiffre**

Lancer : `cd C:\Users\ipmss\chartographer\src-tauri; cargo test`
Attendu : SUCCÈS, y compris les tests existants `day_report_*` de `queries.rs` (ils portent sur les mêmes valeurs qu'avant : le refactor ne doit rien changer).

- [ ] **Étape 12 : commit**

```bash
git add src-tauri/src/store/rankings.rs src-tauri/src/store/mod.rs src-tauri/src/store/queries.rs
git commit -m "Rang d une journee et revenus quotidiens dans un module a part"
```

---

## Tâche 2 : la liste classée complète, côté base

**Fichiers :**
- Modifier : `src-tauri/src/store/rankings.rs`
- Modifier : `src-tauri/src/models.rs` (à la suite de `DayProject`, vers la ligne 138)

**Interfaces :**
- Consomme : `rank_within`, `revenue_by_day`, `RANK_WINDOW_DAYS` (tâche 1), `queries::timeline`.
- Produit : `pub fn day_rankings(conn: &Connection, from: &str, to: &str, filter: PlatformFilter) -> Result<DayRankings>`, et les deux modèles ci-dessous.

- [ ] **Étape 1 : écrire les modèles**

Dans `src-tauri/src/models.rs`, après `DayProject` :

```rust
/// Une journée dans le classement, avec les deux rangs qui la situent.
#[derive(Debug, Clone, Serialize)]
pub struct DayRankRow {
    pub day: String,
    pub modrinth: i64,
    pub curseforge: i64,
    pub total: i64,
    /// Revenus du jour, en dollars, tels que la base les connaît.
    pub revenue: String,
    /// Rang dans la période affichée. Changer les dates le change.
    pub rank_period: Option<i64>,
    /// Rang parmi les quatre-vingt-dix journées qui la précèdent, elle comprise :
    /// le rang qu'elle avait le jour où elle s'est produite.
    pub rank_at_the_time: Option<i64>,
    /// Journées réellement comparées pour ce dernier rang.
    pub compared_days: i64,
}

/// Le classement des journées d'une période, et de quoi le lire sans se tromper.
#[derive(Debug, Clone, Serialize)]
pub struct DayRankings {
    /// Les journées relevées, de la plus ancienne à la plus récente.
    pub rows: Vec<DayRankRow>,
    /// Première journée relevée pour chaque plateforme, toutes périodes
    /// confondues : avant elle, un total ne porte que sur l'autre plateforme.
    pub first_modrinth_day: Option<String>,
    pub first_curseforge_day: Option<String>,
}
```

- [ ] **Étape 2 : écrire les tests qui échouent**

Dans `mod tests` de `rankings.rs` :

```rust
    /// Le rang du jour ne connaît que le passé : une journée énorme survenue
    /// après ne peut pas rétrograder celles d'avant.
    #[test]
    fn the_rank_of_a_day_never_looks_ahead() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(500), None, None).unwrap();

        let out = day_rankings(&conn, "2026-08-10", "2026-08-12", PlatformFilter::default())
            .unwrap();
        let first = out.rows.iter().find(|r| r.day == "2026-08-10").unwrap();
        assert_eq!(first.rank_at_the_time, Some(1), "elle était première le jour même");
        assert_eq!(first.rank_period, Some(2), "mais seconde sur la période");
    }

    /// Le rang sur la période, lui, regarde toute la période.
    #[test]
    fn the_period_rank_orders_the_whole_range() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-11", Some(500), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-12", Some(300), None, None).unwrap();

        let out = day_rankings(&conn, "2026-08-10", "2026-08-13", PlatformFilter::default())
            .unwrap();
        let rank_of = |day: &str| {
            out.rows.iter().find(|r| r.day == day).unwrap().rank_period
        };
        assert_eq!(rank_of("2026-08-11"), Some(1));
        assert_eq!(rank_of("2026-08-12"), Some(2));
        assert_eq!(rank_of("2026-08-10"), Some(3));
    }

    /// Les journées sont rendues dans l'ordre du temps : c'est ce que le
    /// graphique attend, et le tableau les retrie comme il veut.
    #[test]
    fn rows_are_returned_in_chronological_order() {
        let (conn, m, _) = seed();
        upsert_daily(&conn, m, "2026-08-12", Some(300), None, None).unwrap();
        upsert_daily(&conn, m, "2026-08-10", Some(100), None, None).unwrap();

        let out = day_rankings(&conn, "2026-08-10", "2026-08-13", PlatformFilter::default())
            .unwrap();
        let days: Vec<&str> = out.rows.iter().map(|r| r.day.as_str()).collect();
        assert_eq!(days, vec!["2026-08-10", "2026-08-12"]);
    }

    /// Les deux plateformes n'ont pas commencé le même jour : la page doit
    /// pouvoir le dire, sinon les vieux totaux paraissent effondrés.
    #[test]
    fn the_first_day_of_each_platform_is_reported() {
        let (conn, m, c) = seed();
        upsert_daily(&conn, c, "2024-03-19", Some(10), None, None).unwrap();
        upsert_daily(&conn, m, "2025-08-11", Some(20), None, None).unwrap();

        let out = day_rankings(&conn, "2024-03-19", "2026-08-13", PlatformFilter::default())
            .unwrap();
        assert_eq!(out.first_curseforge_day.as_deref(), Some("2024-03-19"));
        assert_eq!(out.first_modrinth_day.as_deref(), Some("2025-08-11"));
    }
```

- [ ] **Étape 3 : lancer les tests et vérifier qu'ils échouent**

Lancer : `cd C:\Users\ipmss\chartographer\src-tauri; cargo test rankings`
Attendu : ÉCHEC, `cannot find function 'day_rankings' in this scope`.

- [ ] **Étape 4 : écrire `day_rankings`**

Ajouter en tête de `rankings.rs` l'import des modèles et de la timeline :

```rust
use crate::models::{DayRankRow, DayRankings};
use crate::store::queries::timeline;
```

Puis, après `revenue_by_day` :

```rust
/// Première journée relevée pour une plateforme, toutes périodes confondues.
fn first_day(conn: &Connection, platform: Platform) -> Result<Option<String>> {
    Ok(conn.query_row(
        "SELECT MIN(m.day) FROM metrics_daily m JOIN projects p ON p.id = m.project_id
         WHERE p.platform = ?1 AND COALESCE(m.downloads, 0) > 0",
        params![platform.as_str()],
        |r| r.get::<_, Option<String>>(0),
    )?)
}

/// Classement des journées d'une période, `to` exclu.
///
/// Chaque journée porte deux rangs, parce que deux questions différentes se
/// posent à son sujet. Le rang sur la période répond à « où se situe-t-elle
/// dans ce que je regarde » et change avec les dates choisies. Le rang du jour
/// répond à « était-ce un bon jour quand il s'est produit » : il ne compare
/// qu'aux quatre-vingt-dix journées qui la précèdent, et rien de ce qui est
/// arrivé ensuite ne peut plus le modifier.
///
/// La fenêtre du second rang déborde la période demandée : les journées qui
/// précèdent le premier jour affiché servent de comparaison sans figurer dans
/// la liste.
pub fn day_rankings(
    conn: &Connection,
    from: &str,
    to: &str,
    filter: PlatformFilter,
) -> Result<DayRankings> {
    let history_start = shift_day(from, -(RANK_WINDOW_DAYS - 1));
    let history = timeline(conn, &history_start, to, filter)?;
    let revenue = revenue_by_day(conn, from, to, filter)?;

    let totals: Vec<i64> = history.iter().map(|p| p.modrinth + p.curseforge).collect();

    // Les rangs sur la période se lisent d'un tri unique : trier chaque ligne
    // séparément coûterait un parcours complet par journée.
    let mut period_totals: Vec<i64> = history
        .iter()
        .zip(&totals)
        .filter(|(p, total)| p.day.as_str() >= from && **total > 0)
        .map(|(_, total)| *total)
        .collect();
    period_totals.sort_unstable_by(|a, b| b.cmp(a));

    let mut rows = Vec::new();
    // Borne basse glissante de la fenêtre : elle n'avance jamais en arrière, ce
    // qui laisse le parcours linéaire au lieu de rouvrir la fenêtre à chaque jour.
    let mut start = 0usize;
    for (i, point) in history.iter().enumerate() {
        let window_start = shift_day(&point.day, -(RANK_WINDOW_DAYS - 1));
        while history[start].day < window_start {
            start += 1;
        }
        if point.day.as_str() < from {
            continue;
        }
        let window: Vec<i64> = totals[start..=i].iter().copied().filter(|t| *t > 0).collect();
        let total = totals[i];
        let (modrinth_revenue, curseforge_revenue) =
            revenue.get(&point.day).copied().unwrap_or_default();
        rows.push(DayRankRow {
            day: point.day.clone(),
            modrinth: point.modrinth,
            curseforge: point.curseforge,
            total,
            revenue: (modrinth_revenue + curseforge_revenue).normalize().to_string(),
            rank_period: rank_within(&period_totals, total),
            rank_at_the_time: rank_within(&window, total),
            compared_days: window.len() as i64,
        });
    }

    Ok(DayRankings {
        rows,
        first_modrinth_day: first_day(conn, Platform::Modrinth)?,
        first_curseforge_day: first_day(conn, Platform::CurseForge)?,
    })
}
```

- [ ] **Étape 5 : lancer les tests et vérifier qu'ils passent**

Lancer : `cd C:\Users\ipmss\chartographer\src-tauri; cargo test rankings`
Attendu : SUCCÈS, 9 tests.

- [ ] **Étape 6 : commit**

```bash
git add src-tauri/src/store/rankings.rs src-tauri/src/models.rs
git commit -m "Classement des journees d une periode"
```

---

## Tâche 3 : exposer la commande au front

**Fichiers :**
- Modifier : `src-tauri/src/commands.rs` (à la suite de `day_report`, ligne 219)
- Modifier : `src-tauri/src/lib.rs:81`
- Modifier : `src/lib/types.ts` (à la suite de `DayReport`, ligne 354)
- Modifier : `src/lib/api.ts:60`

**Interfaces :**
- Consomme : `rankings::day_rankings` (tâche 2), `queries::resolve_range`, `queries::PlatformFilter`.
- Produit : commande Tauri `day_rankings(rangeDays, from, to, platforms)` et `api.dayRankings(...)` côté TypeScript.

- [ ] **Étape 1 : écrire la commande**

Dans `src-tauri/src/commands.rs`, juste après `day_report` :

```rust
/// Classement des journées de la période choisie.
///
/// Les bornes se résolvent comme celles de la page de vision : la page de
/// classement partage son sélecteur de dates, et deux règles de fenêtre
/// donneraient deux périodes pour un même réglage.
#[tauri::command]
pub fn day_rankings(
    state: State<'_, AppState>,
    range_days: i64,
    from: Option<String>,
    to: Option<String>,
    platforms: Option<Vec<String>>,
) -> Result<crate::models::DayRankings> {
    let today = sync::today_utc();
    let range = range_days.clamp(7, 730);
    let (from, to) = queries::resolve_range(&today, range, from.as_deref(), to.as_deref());
    let filter = queries::PlatformFilter::from_names(platforms.as_deref());
    state
        .store
        .with(|conn| crate::store::rankings::day_rankings(conn, &from, &to, filter))
}
```

- [ ] **Étape 2 : enregistrer la commande**

Dans `src-tauri/src/lib.rs`, à la suite de `commands::day_report,` (ligne 81) :

```rust
            commands::day_rankings,
```

- [ ] **Étape 3 : compiler**

Lancer : `cd C:\Users\ipmss\chartographer\src-tauri; cargo build`
Attendu : SUCCÈS. Si la compilation bute sur le TLS du proxy, poser `$env:CARGO_HTTP_CHECK_REVOKE = "false"` dans la session avant de relancer.

- [ ] **Étape 4 : écrire les types du front**

Dans `src/lib/types.ts`, après `DayReport` :

```ts
/** Une journée dans le classement, avec les deux rangs qui la situent. */
export interface DayRankRow {
  day: string;
  modrinth: number;
  curseforge: number;
  total: number;
  /** Revenus du jour, en dollars, tels que la base les connaît. */
  revenue: string;
  /** Rang dans la période affichée : changer les dates le change. */
  rank_period: number | null;
  /** Rang qu'avait la journée le jour où elle s'est produite. */
  rank_at_the_time: number | null;
  compared_days: number;
}

export interface DayRankings {
  /** Les journées relevées, de la plus ancienne à la plus récente. */
  rows: DayRankRow[];
  first_modrinth_day: string | null;
  first_curseforge_day: string | null;
}
```

- [ ] **Étape 5 : écrire l'appel**

Dans `src/lib/api.ts`, ajouter `DayRankings` à la liste des types importés en tête de fichier, puis, sous `dayReport` (ligne 60) :

```ts
  dayRankings: (rangeDays: number, from: string | null, to: string | null, platforms: string[]) =>
    invoke<DayRankings>("day_rankings", { rangeDays, from, to, platforms }),
```

- [ ] **Étape 6 : vérifier les types**

Lancer : `cd C:\Users\ipmss\chartographer; npm run check`
Attendu : aucune erreur nouvelle.

- [ ] **Étape 7 : commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src/lib/types.ts src/lib/api.ts
git commit -m "Commande day_rankings et son appel cote interface"
```

---

## Tâche 4 : les deux graphiques

**Fichiers :**
- Créer : `src/lib/charts/dayRanking.ts`
- Créer : `src/lib/charts/dayRanking.test.ts`

**Interfaces :**
- Consomme : `DayRankRow` (tâche 3), `charts/theme` (`axisStyle`, `BASE_GRID`, `DARK`, `dayAxis`, `dayTooltip`, `Palette`), `components/rank` (`PODIUM`).
- Produit :
  - `export function dailyBarsOption(rows: DayRankRow[], p?: Palette)` — barres empilées par plateforme, les trois meilleures journées de la période marquées aux couleurs du podium.
  - `export function rankCurveOption(rows: DayRankRow[], p?: Palette)` — la courbe du rang du jour, axe inversé pour que le premier rang soit en haut.

- [ ] **Étape 1 : écrire les tests qui échouent**

Créer `src/lib/charts/dayRanking.test.ts` :

```ts
import { describe, expect, it } from "vitest";
import { PODIUM } from "../components/rank";
import type { DayRankRow } from "../types";
import { dailyBarsOption, rankCurveOption } from "./dayRanking";

function row(partial: Partial<DayRankRow>): DayRankRow {
  return {
    day: "2026-08-10",
    modrinth: 0,
    curseforge: 0,
    total: 0,
    revenue: "0",
    rank_period: null,
    rank_at_the_time: null,
    compared_days: 0,
    ...partial,
  };
}

const rows = [
  row({ day: "2026-08-09", modrinth: 40, curseforge: 10, total: 50, rank_period: 2, rank_at_the_time: 1 }),
  row({ day: "2026-08-10", modrinth: 80, curseforge: 20, total: 100, rank_period: 1, rank_at_the_time: 1 }),
];

describe("dailyBarsOption", () => {
  it("porte une série par plateforme, dans l'ordre des jours", () => {
    const option = dailyBarsOption(rows);
    const modrinth = option.series.find((s) => s.id === "day:modrinth");
    expect(modrinth?.data.map((d) => d.value)).toEqual([40, 80]);
    expect(option.xAxis.data).toEqual(["2026-08-09", "2026-08-10"]);
  });

  it("marque la meilleure journée de la période à la couleur du podium", () => {
    const option = dailyBarsOption(rows);
    const curseforge = option.series.find((s) => s.id === "day:curseforge");
    // Le liseré du podium coiffe la pile, donc la série du haut.
    expect(curseforge?.data[1].itemStyle?.borderColor).toBe(PODIUM[0]);
    expect(curseforge?.data[0].itemStyle?.borderColor).toBe(PODIUM[1]);
  });
});

describe("rankCurveOption", () => {
  it("met le premier rang en haut", () => {
    const option = rankCurveOption(rows);
    expect(option.yAxis.inverse).toBe(true);
    expect(option.yAxis.min).toBe(1);
  });

  it("laisse un trou pour les journées sans rang", () => {
    const option = rankCurveOption([...rows, row({ day: "2026-08-11" })]);
    expect(option.series[0].data[2]).toBeNull();
  });
});
```

- [ ] **Étape 2 : lancer les tests et vérifier qu'ils échouent**

Lancer : `cd C:\Users\ipmss\chartographer; npx vitest run src/lib/charts/dayRanking.test.ts`
Attendu : ÉCHEC, `Failed to resolve import "./dayRanking"`.

- [ ] **Étape 3 : écrire les deux fabriques**

Créer `src/lib/charts/dayRanking.ts` :

```ts
import { PODIUM } from "../components/rank";
import type { DayRankRow } from "../types";
import { axisStyle, BASE_GRID, DARK, dayAxis, dayTooltip, type Palette } from "./theme";

/** Zoom commun aux deux vues : un an de journées ne tient pas à l'écran. */
const ZOOM = [
  { type: "inside", start: 0, end: 100 },
  { type: "slider", height: 20, bottom: 8 },
];

/**
 * Les journées de la période, barre par barre.
 *
 * Le podium est marqué d'un liseré plutôt que d'une couleur pleine : la barre
 * dit déjà la plateforme, et la repeindre ferait perdre cette lecture-là pour
 * en gagner une autre.
 */
export function dailyBarsOption(rows: DayRankRow[], p: Palette = DARK) {
  const axis = axisStyle(p);
  const crown = (row: DayRankRow, top: boolean) => {
    const color = row.rank_period !== null ? PODIUM[row.rank_period - 1] : undefined;
    return top && color ? { itemStyle: { borderColor: color, borderWidth: 2 } } : {};
  };
  return {
    grid: BASE_GRID,
    tooltip: dayTooltip(p, { sorted: true }),
    legend: { data: ["Modrinth", "CurseForge"], textStyle: { color: p.textDim }, top: 0 },
    xAxis: dayAxis(rows.map((r) => r.day), p),
    yAxis: { type: "value", ...axis },
    dataZoom: ZOOM.map((z) => ({ ...z, borderColor: p.grid, textStyle: { color: p.textDim } })),
    series: [
      {
        id: "day:modrinth",
        name: "Modrinth",
        type: "bar",
        stack: "jour",
        itemStyle: { color: p.modrinth },
        data: rows.map((r) => ({ value: r.modrinth, ...crown(r, r.curseforge === 0) })),
      },
      {
        id: "day:curseforge",
        name: "CurseForge",
        type: "bar",
        stack: "jour",
        itemStyle: { color: p.curseforge },
        data: rows.map((r) => ({ value: r.curseforge, ...crown(r, r.curseforge > 0) })),
      },
    ],
  };
}

/**
 * Le rang qu'avait chaque journée le jour où elle s'est produite.
 *
 * L'axe est retourné : un premier rang est un sommet, et le lire au fond du
 * graphique demanderait au lecteur de renverser mentalement toute la courbe.
 * Les journées sans relevé laissent un trou plutôt qu'un zéro, qui se lirait
 * comme un rang.
 */
export function rankCurveOption(rows: DayRankRow[], p: Palette = DARK) {
  const axis = axisStyle(p);
  return {
    grid: BASE_GRID,
    tooltip: dayTooltip(p),
    xAxis: dayAxis(rows.map((r) => r.day), p),
    yAxis: { type: "value", inverse: true, min: 1, ...axis },
    dataZoom: ZOOM.map((z) => ({ ...z, borderColor: p.grid, textStyle: { color: p.textDim } })),
    series: [
      {
        id: "day:rank",
        name: "Rang du jour",
        type: "line",
        step: "middle",
        showSymbol: false,
        connectNulls: false,
        itemStyle: { color: p.accent },
        data: rows.map((r) => r.rank_at_the_time),
      },
    ],
  };
}
```

Avant d'écrire, ouvrir `src/lib/charts/theme.ts` et vérifier le nom exact du champ d'accent de `Palette` (`p.accent` ci-dessus) ainsi que la signature de `dayTooltip` ; les recopier tels quels plutôt que de les supposer.

- [ ] **Étape 4 : lancer les tests et vérifier qu'ils passent**

Lancer : `cd C:\Users\ipmss\chartographer; npx vitest run src/lib/charts/dayRanking.test.ts`
Attendu : SUCCÈS, 4 tests.

- [ ] **Étape 5 : commit**

```bash
git add src/lib/charts/dayRanking.ts src/lib/charts/dayRanking.test.ts
git commit -m "Graphiques du classement des journees"
```

---

## Tâche 5 : la page

**Fichiers :**
- Créer : `src/lib/views/detail/DaysDetail.svelte`

**Interfaces :**
- Consomme : `api.dayRankings` (tâche 3), `dailyBarsOption` et `rankCurveOption` (tâche 4), `DetailShell`, `StatRow`, `RankedTable`, `Hint`, `Chart`, `format` (`compactNumber`, `formatDay`, `formatDayLong`, `formatMoney`), `dashboard`, `theme`.
- Produit : le composant `DaysDetail`, monté par `App.svelte` (tâche 6).

**Points à respecter :**
- Le `?` vient de `Hint` — aucun bouton d'aide écrit à la main.
- Le tableau vient de `RankedTable` — aucune balise `<table>` propre à cette page. `ranked={false}` : la colonne de rang de `RankedTable` numérote les lignes affichées, alors que cette page montre deux rangs qui lui sont propres.
- `DetailShell` apporte déjà le `RangePicker` : la page se recharge quand les bornes changent.

- [ ] **Étape 1 : écrire la page**

Créer `src/lib/views/detail/DaysDetail.svelte` :

```svelte
<script lang="ts">
  /**
   * Le classement des journées.
   *
   * La page Journée juge une journée à la fois ; celle-ci les met en rang.
   * Deux rangs, plutôt qu'un : celui de la période regardée, qui bouge avec
   * les dates choisies, et celui que la journée avait le jour même, qui ne
   * bougera plus jamais. Les confondre ferait croire qu'un mois faible
   * rétrograde des journées vieilles d'un an.
   */
  import { api } from "../../api";
  import Chart from "../../charts/Chart.svelte";
  import { dailyBarsOption, rankCurveOption } from "../../charts/dayRanking";
  import { palette } from "../../charts/theme";
  import Hint from "../../components/Hint.svelte";
  import RankedTable from "../../components/RankedTable.svelte";
  import { PODIUM } from "../../components/rank";
  import StatRow from "../../components/StatRow.svelte";
  import { compactNumber, formatDay, formatDayLong, formatMoney } from "../../format";
  import { dashboard } from "../../state.svelte";
  import { theme } from "../../theme.svelte";
  import type { AppErrorPayload, DayRankings } from "../../types";
  import DetailShell from "./DetailShell.svelte";

  let data = $state<DayRankings | null>(null);
  let loading = $state(false);
  let mode = $state<"jours" | "rang">("jours");
  let order = $state<"rang" | "date">("rang");

  // Les bornes et les plateformes visibles commandent le classement : il se
  // relève dès que l'une d'elles change.
  $effect(() => {
    const [days, from, to, platforms] = [
      dashboard.rangeDays,
      dashboard.rangeFrom,
      dashboard.rangeTo,
      dashboard.visiblePlatforms,
    ];
    loading = true;
    api
      .dayRankings(days, from, to, platforms)
      .then((value) => (data = value))
      .catch((e) => (dashboard.error = (e as AppErrorPayload)?.message ?? String(e)))
      .finally(() => (loading = false));
  });

  const rows = $derived(data?.rows ?? []);
  const relevés = $derived(rows.filter((r) => r.total > 0));
  const option = $derived(
    mode === "jours" ? dailyBarsOption(rows, palette(theme.dark)) : rankCurveOption(rows, palette(theme.dark)),
  );

  const best = $derived(relevés.find((r) => r.rank_period === 1) ?? null);
  const total = $derived(relevés.reduce((sum, r) => sum + r.total, 0));
  const average = $derived(relevés.length ? Math.round(total / relevés.length) : 0);
  /**
   * Journées qui furent premières le jour de leur passage : ce sont les
   * records successifs, la seule lecture qui distingue une bonne journée d'un
   * sommet.
   */
  const sommets = $derived(relevés.filter((r) => r.rank_at_the_time === 1).length);

  const listed = $derived(
    order === "rang"
      ? [...relevés].sort((a, b) => (a.rank_period ?? 0) - (b.rank_period ?? 0))
      : [...relevés].sort((a, b) => (a.day < b.day ? 1 : -1)),
  );

  const PERIOD =
    "Rang de la journée parmi celles de la période affichée, la meilleure en tête. Changer les dates change ce rang : il ne dit rien d'autre que la place tenue dans ce qui est montré à l'écran.";
  const AT_THE_TIME =
    "Rang de la journée parmi les quatre-vingt-dix qui la précèdent, celle-ci comprise — le rang qu'elle avait le jour où elle s'est produite. Le classement ne regarde jamais en avant, et rien de ce qui est arrivé ensuite ne peut plus le changer. Les journées sans aucun relevé sont écartées, elles flatteraient le rang.";
  const REVENUE =
    "Modrinth relève ses revenus jour par jour. CurseForge n'en publie aucun : ce qui apparaît ici vient de l'écart entre deux soldes de points, relevés au passage seulement, si bien que la plupart des journées n'en portent aucun.";
  const coverage = $derived(
    `Modrinth est relevé depuis le ${data?.first_modrinth_day ? formatDayLong(data.first_modrinth_day) : "—"}, CurseForge depuis le ${data?.first_curseforge_day ? formatDayLong(data.first_curseforge_day) : "—"}. Avant ces dates, un total ne porte que sur l'autre plateforme : il paraît faible sans l'être.`,
  );
</script>

<DetailShell
  title="Classement des journées"
  subtitle="{relevés.length} journées relevées{loading ? " · relevé en cours…" : ""}"
>
  {#snippet actions()}
    <div class="switch">
      <button class:active={mode === "jours"} onclick={() => (mode = "jours")}>Par jour</button>
      <button class:active={mode === "rang"} onclick={() => (mode = "rang")}>Rang au fil du temps</button>
    </div>
  {/snippet}

  <StatRow
    stats={[
      { label: "Journées relevées", value: String(relevés.length) },
      { label: "Moyenne par jour", value: compactNumber(average) },
      {
        label: "Meilleure journée",
        value: best ? compactNumber(best.total) : "—",
        hint: best ? formatDay(best.day) : "aucun relevé",
      },
      { label: "Journées record", value: String(sommets), hint: "premières le jour même" },
    ]}
  />

  <div class="chart">
    <h2>
      {mode === "jours" ? "Téléchargements par journée" : "Rang au fil du temps"}
      <Hint text={mode === "jours" ? PERIOD : AT_THE_TIME} />
      <Hint text={coverage} />
    </h2>
    <Chart {option} height={420} />
  </div>

  <div class="panel">
    <h2>
      Les journées, une par une
      <Hint text={AT_THE_TIME} />
      <span class="spacer"></span>
      <span class="switch small">
        <button class:active={order === "rang"} onclick={() => (order = "rang")}>Par rang</button>
        <button class:active={order === "date"} onclick={() => (order = "date")}>Par date</button>
      </span>
    </h2>
    {#if listed.length === 0}
      <p class="empty">Aucune journée relevée sur cette période.</p>
    {:else}
      <RankedTable
        ranked={false}
        maxHeight={520}
        columns={[
          { label: "Journée", align: "left" },
          { label: "Rang période" },
          { label: "Rang du jour" },
          { label: "Modrinth" },
          { label: "CurseForge" },
          { label: "Total" },
          { label: "Revenus" },
        ]}
        rows={listed}
        key={(row) => row.day}
      >
        {#snippet cells(row)}
          {@const podium = row.rank_period !== null ? PODIUM[row.rank_period - 1] : null}
          <td class="left">{formatDayLong(row.day)}</td>
          <td>
            <span class="badge" class:podium={podium !== null} style="--rank: {podium ?? ''}">
              {row.rank_period ?? "—"}
            </span>
          </td>
          <td class="dim">
            {row.rank_at_the_time === null ? "—" : `${row.rank_at_the_time} / ${row.compared_days}`}
          </td>
          <td>{compactNumber(row.modrinth)}</td>
          <td>{compactNumber(row.curseforge)}</td>
          <td class="strong">{compactNumber(row.total)}</td>
          <td class="dim">{formatMoney(row.revenue)}</td>
        {/snippet}
      </RankedTable>
      <p class="foot">
        Revenus du jour
        <Hint text={REVENUE} />
      </p>
    {/if}
  </div>
</DetailShell>

<style>
  .switch {
    display: flex;
    gap: 4px;
  }
  .switch button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 7px;
    padding: 5px 12px;
    font: inherit;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .switch.small button {
    padding: 3px 9px;
    font-size: 0.74rem;
  }
  .switch button.active,
  .switch button:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .chart,
  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
  }
  .panel {
    margin-top: 14px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 10px;
    font-size: 0.9rem;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  /* Le rang de la période reprend la pastille des tableaux classés : c'est la
   * même idée, elle doit se lire pareil. */
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    padding: 1px 5px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-dim);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .badge.podium {
    background: color-mix(in srgb, var(--rank) 22%, transparent);
    color: var(--rank);
    font-weight: 600;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 10px 0 0;
    font-size: 0.78rem;
    color: var(--text-dim);
  }
  .empty {
    margin: 10px 0 0;
    color: var(--text-dim);
    font-size: 0.84rem;
  }
</style>
```

Deux vérifications à faire en écrivant : `RankedTable` attend `cells` comme `Snippet<[Row, number]>` — n'en déclarer qu'un paramètre est permis ; et `theme.svelte.ts` expose bien `theme.dark` (le relire au besoin).

- [ ] **Étape 2 : vérifier les types**

Lancer : `cd C:\Users\ipmss\chartographer; npm run check`
Attendu : aucune erreur nouvelle. Une erreur sur `relevés` en nom de variable est possible selon la configuration : le cas échéant, renommer en `measured`, sans toucher aux libellés visibles.

- [ ] **Étape 3 : commit**

```bash
git add src/lib/views/detail/DaysDetail.svelte
git commit -m "Page du classement des journees"
```

---

## Tâche 6 : brancher la page et son accès

**Fichiers :**
- Modifier : `src/lib/state.svelte.ts:18-25`
- Modifier : `src/App.svelte:143-162`
- Modifier : `src/lib/views/Day.svelte:82-121`

**Interfaces :**
- Consomme : `DaysDetail` (tâche 5).
- Produit : `dashboard.openDetail("days")` ouvre la page ; le rang de la page Journée y mène.

- [ ] **Étape 1 : ajouter la vue à la liste**

Dans `src/lib/state.svelte.ts`, ligne 18 :

```ts
export type DetailView =
  | "timeline"
  | "countries"
  | "platforms"
  | "loaders"
  | "events"
  | "projects"
  | "followers"
  | "days";
```

- [ ] **Étape 2 : monter la page**

Dans `src/App.svelte`, importer le composant à côté des autres vues de détail :

```ts
  import DaysDetail from "./lib/views/detail/DaysDetail.svelte";
```

puis, dans le bloc des vues plein écran, à la suite de `FollowersDetail` (ligne 147) — le classement ne vient pas de l'aperçu, il se relève lui-même :

```svelte
  {:else if dashboard.detail === "days"}
    <DaysDetail />
```

- [ ] **Étape 3 : ouvrir la page depuis la page Journée**

Dans `src/lib/views/Day.svelte`, remplacer le bloc du rang (lignes 116-121) par un bouton qui mène au classement :

```svelte
    {#if report.rank !== null}
      <button class="rank" onclick={() => dashboard.openDetail("days")}>
        {report.rank}<sup>{report.rank === 1 ? "re" : "e"}</sup> journée sur {report.ranked_days}
        <span class="more">voir le classement</span>
      </button>
      <Hint text={RANK} />
    {/if}
```

Le `Hint` sort du bouton : imbriquer un bouton d'aide dans un bouton cliquable donne un balisage invalide et un clic ambigu.

Ajouter au style de la page, à la suite de la règle `.rank` existante (ligne 321) :

```css
  .rank {
    border: 0;
    background: none;
    padding: 0;
  }
  .rank:hover .more {
    color: var(--accent);
  }
  .more {
    font-size: 0.72rem;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
```

- [ ] **Étape 4 : vérifier**

Lancer : `cd C:\Users\ipmss\chartographer; npm run check; npm run test`
Attendu : aucune erreur de type, tous les tests au vert.

- [ ] **Étape 5 : commit**

```bash
git add src/lib/state.svelte.ts src/App.svelte src/lib/views/Day.svelte
git commit -m "Acces au classement depuis la page Journee"
```

---

## Tâche 7 : vérifier sur les vraies données

Un test qui passe sur une base de test ne prouve rien de ce que la page affichera. Cette tâche confronte le calcul à la base réelle du poste, puis à l'application lancée.

**Fichiers :**
- Créer (hors dépôt) : `%TEMP%\claude\...\scratchpad\verif_classement.py`

- [ ] **Étape 1 : recalculer le classement en dehors de l'application**

Écrire le script de contrôle, qui refait le rang rétrospectif en SQL pur sur la base réelle :

```python
import sqlite3, os, datetime
db = os.path.expandvars(r"%APPDATA%\fr.dreykaoas.chartographer\chartographer.db")
c = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
jours = dict(c.execute(
    "SELECT m.day, SUM(COALESCE(m.downloads,0)) FROM metrics_daily m GROUP BY m.day"
).fetchall())
jours = {d: t for d, t in jours.items() if t > 0}
serie = sorted(jours)
print("journées relevées :", len(serie), serie[0], "→", serie[-1])
def rang(jour):
    debut = (datetime.date.fromisoformat(jour) - datetime.timedelta(days=89)).isoformat()
    fenetre = [t for d, t in jours.items() if debut <= d <= jour]
    return sum(1 for t in fenetre if t > jours[jour]) + 1, len(fenetre)
for jour in serie[-3:]:
    print(jour, jours[jour], "rang du jour :", rang(jour))
records = [j for j in serie if rang(j)[0] == 1]
print("journées premières le jour même :", len(records))
print("meilleure journée absolue :", max(jours.items(), key=lambda kv: kv[1]))
```

Lancer : `python "<chemin>\verif_classement.py"`
Attendu : des chiffres, pas une erreur. Noter la sortie : elle sert de référence.

- [ ] **Étape 2 : lancer l'application et comparer**

Lancer : `cd C:\Users\ipmss\chartographer; npm run tauri dev`
Ouvrir l'onglet Journée, cliquer sur le rang, et vérifier point par point :
1. Le nombre de journées relevées annoncé en sous-titre correspond à la sortie de l'étape 1 (au filtre de plateforme près : le contrôle ci-dessus ne filtre pas).
2. La meilleure journée affichée est bien celle du script.
3. Le rang du jour de la journée d'hier est identique à celui qu'affiche la page Journée pour cette même date.
4. Le nombre de journées record correspond.
5. Basculer sur « Rang au fil du temps » : la courbe touche 1 aux dates records du script, et laisse un trou aux dates sans relevé.
6. Masquer CurseForge dans la barre du haut : les totaux et les rangs se recalculent, et les journées antérieures au premier relevé Modrinth disparaissent du classement.
7. Survoler chaque `?` : les quatre bulles s'ouvrent, tiennent dans la fenêtre, et disent bien ce que la colonne signifie.
8. Changer la période avec le sélecteur de dates : le rang période bouge, le rang du jour ne bouge pas. C'est le point qui prouve que les deux rangs sont bien deux choses différentes.

Écrire ce qui a été constaté. Si un chiffre diverge, ne pas ajuster la page : reprendre le calcul, la divergence est le symptôme.

- [ ] **Étape 3 : dernier passage complet**

Lancer : `cd C:\Users\ipmss\chartographer; npm run check; npm run test`
puis : `cd C:\Users\ipmss\chartographer\src-tauri; cargo test`
Attendu : tout au vert, aucune erreur de type.

- [ ] **Étape 4 : commit et push**

```bash
git add -A
git commit -m "Verification du classement sur la base reelle"
git push
```

Le dépôt est `Dreyka-Oas/chartographer` : pousser avec ce compte-là (un mauvais compte rend 404, pas 403).

---

## Ce que ce plan ne fait pas

- **Pas de nouvel appel réseau.** Tout le classement se calcule sur ce qui est déjà en base. Aucune limite d'API n'est approchée.
- **Pas de rattrapage d'historique CurseForge.** La plateforme n'expose pas de statistiques auteur par jour ; l'application les tient de son tableau de bord et de ses propres snapshots. Le seul moyen d'aller plus loin serait d'importer le CSV du tableau de bord auteur, ce qui est un autre sujet et mérite son propre plan.
- **Pas de classement par mod.** La page classe des journées. Le classement des mods existe déjà sur la page « Téléchargements par jour ».
