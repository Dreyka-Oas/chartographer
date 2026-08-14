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
