use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Modrinth,
    CurseForge,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Modrinth => "modrinth",
            Platform::CurseForge => "curseforge",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "curseforge" => Platform::CurseForge,
            _ => Platform::Modrinth,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Kpis {
    pub downloads_total: i64,
    pub downloads_modrinth: i64,
    pub downloads_curseforge: i64,
    pub downloads_30d: i64,
    /// Même fenêtre de trente jours, plateforme par plateforme.
    pub downloads_30d_modrinth: i64,
    pub downloads_30d_curseforge: i64,
    pub downloads_prev_30d: i64,
    /// Tout ce que les deux plateformes ont rapporté depuis l'origine :
    /// reversement Modrinth et contre-valeur des points CurseForge additionnés.
    pub revenue_total: String,
    /// Part Modrinth : déjà retiré, retirable et encore en maturation.
    pub revenue_modrinth: String,
    /// Part CurseForge : le solde de points converti au tarif publié.
    pub revenue_curseforge: String,
    /// Somme retirable immédiatement.
    pub revenue_available: String,
    /// Ce qui, dans cette somme, vient de chaque plateforme : le solde de
    /// reversement Modrinth d'un côté, les points CurseForge de l'autre.
    pub revenue_available_modrinth: String,
    pub revenue_available_curseforge: String,
    /// Somme gagnée mais encore en maturation.
    pub revenue_pending: String,
    /// Revenus relevés jour par jour sur la fenêtre affichée. Les analytics ne
    /// remontent pas jusqu'à l'origine : ce total est inférieur au cumul.
    pub revenue_window: String,
    /// Téléchargements de la période choisie dans la barre de filtres, et de la
    /// période de même durée qui la précède, pour l'écart.
    pub range_downloads: i64,
    pub range_downloads_modrinth: i64,
    pub range_downloads_curseforge: i64,
    pub range_downloads_prev: i64,
    /// Revenus de la même période. Modrinth les relève jour par jour ; ceux de
    /// CurseForge sont reconstruits par écart entre deux soldes de points.
    pub range_revenue: String,
    pub range_revenue_modrinth: String,
    pub range_revenue_curseforge: String,
    pub followers: i64,
    /// Abonnés par plateforme. CurseForge ne les publie nulle part : ils sont
    /// relevés sur le tableau de bord auteur, comme les téléchargements.
    pub followers_modrinth: i64,
    pub followers_curseforge: i64,
    pub projects_active: i64,
    /// Projets actifs par plateforme. CurseForge n'expose aucun abonné : c'est
    /// le seul décompte qui puisse y être ventilé.
    pub projects_modrinth: i64,
    pub projects_curseforge: i64,
}

/// Bilan d'une seule journée.
///
/// La question n'est pas « combien », mais « était-ce un bon jour » : un
/// chiffre seul n'y répond pas, il lui faut ceux d'à côté. On rend donc la
/// journée, celle qui la précède, les moyennes récentes et le rang du jour
/// parmi les précédents — de quoi juger sans avoir à chercher ailleurs.
#[derive(Debug, Clone, Serialize)]
pub struct DayReport {
    pub day: String,
    /// Vrai quand la journée n'est pas finie : ses chiffres monteront encore.
    pub partial: bool,
    pub downloads: DayFigure,
    pub revenue: DayMoney,
    /// Rang du jour parmi les journées relevées, 1 étant la meilleure.
    pub rank: Option<i64>,
    /// Nombre de journées comparées, rang compris.
    pub ranked_days: i64,
    /// Meilleure journée connue, pour situer celle-ci.
    pub best_day: Option<String>,
    pub best_downloads: i64,
    /// Abonnés gagnés ou perdus ce jour-là, si les deux relevés existent.
    pub followers_delta: Option<i64>,
    /// Projets qui ont porté la journée, du plus fort au plus faible.
    pub projects: Vec<DayProject>,
    pub events: Vec<EventRow>,
}

/// Une mesure du jour, ses parts, et ce à quoi la comparer.
#[derive(Debug, Clone, Serialize)]
pub struct DayFigure {
    pub total: i64,
    pub modrinth: i64,
    pub curseforge: i64,
    /// La veille, pour l'écart immédiat.
    pub previous: i64,
    /// Moyenne des sept et des vingt-huit journées qui précèdent, la journée
    /// jugée exclue : se comparer à soi-même fausserait le verdict.
    pub average_7: f64,
    pub average_28: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayMoney {
    pub total: String,
    pub modrinth: String,
    pub curseforge: String,
    pub previous: String,
    pub average_7: String,
    pub average_28: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayProject {
    pub key: String,
    pub title: String,
    pub icon_url: Option<String>,
    pub modrinth: i64,
    pub curseforge: i64,
    pub total: i64,
    /// Même total la veille, pour voir ce qui a bougé.
    pub previous: i64,
}

/// Une journée dans le classement, avec le rang que les réglages ont produit.
#[derive(Debug, Clone, Serialize)]
pub struct DayRankRow {
    pub day: String,
    pub modrinth: i64,
    pub curseforge: i64,
    pub total: i64,
    /// Revenus du jour, en dollars, tels que la base les connaît.
    pub revenue: String,
    /// Rang de la journée selon les réglages demandés, 1 étant le meilleur.
    pub rank: Option<i64>,
    /// Journées réellement comparées pour établir ce rang.
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

/// Ce sur quoi les journées se classent.
///
/// Le classement porte sur les téléchargements par défaut. Les revenus sont
/// proposés, mais ils ne racontent pas la même chose : Modrinth les relève au
/// jour le jour quand CurseForge n'en publie aucun, si bien qu'un classement
/// par revenus est d'abord un classement Modrinth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RankBy {
    Downloads,
    Revenue,
}

impl Default for RankBy {
    fn default() -> Self {
        Self::Downloads
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelinePoint {
    pub day: String,
    pub modrinth: i64,
    pub curseforge: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub key: String,
    pub title: String,
    pub icon_url: Option<String>,
    pub modrinth_id: Option<i64>,
    pub curseforge_id: Option<i64>,
    /// Identifiants tels que les plateformes les connaissent : ceux-là seuls
    /// servent à leur parler, les précédents ne valent qu'en base.
    pub modrinth_ext_id: Option<String>,
    pub curseforge_ext_id: Option<i64>,
    pub modrinth_downloads: i64,
    pub curseforge_downloads: i64,
    pub followers: i64,
    pub link_confidence: Option<f64>,
    pub spark: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountryTotal {
    pub country: String,
    pub downloads: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoaderCell {
    pub game_version: String,
    pub loader: String,
    pub downloads: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevenuePoint {
    pub day: String,
    pub amount: String,
}

/// Une échéance de l'échéancier de reversement Modrinth.
#[derive(Debug, Clone, Serialize)]
pub struct PayoutPoint {
    pub date: String,
    pub amount: String,
    /// Vrai si l'échéance est postérieure à aujourd'hui : revenu à venir.
    pub future: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Payout {
    pub available: String,
    pub pending: String,
    pub withdrawn_lifetime: String,
    pub withdrawn_ytd: String,
    pub schedule: Vec<PayoutPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevenueByProject {
    pub key: String,
    pub title: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventRow {
    pub occurred_at: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Freshness {
    pub provider: String,
    pub status: String,
    pub finished_at: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionRow {
    pub version_number: Option<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub downloads: i64,
    pub date_published: Option<String>,
}

/// Tout ce qu'une vue détaillée de projet affiche, aligné sur un axe de jours dense.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectDetail {
    pub summary: ProjectSummary,
    pub days: Vec<String>,
    pub downloads: Vec<i64>,
    pub views: Vec<i64>,
    pub curseforge: Vec<i64>,
    pub revenue: Vec<String>,
    pub countries: Vec<CountryTotal>,
    pub versions: Vec<VersionRow>,
}

/// Ce que le tableau de bord CurseForge rapporte de son côté argent.
///
/// Les points sont la monnaie du programme ; les montants mensuels et les deux
/// estimations sont annoncés en dollars par le site lui-même.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CfRevenue {
    pub points: i64,
    pub points_usd: String,
    pub last_month: Option<String>,
    pub year_to_date: Option<String>,
    pub monthly: Vec<crate::store::metrics::CfRevenueEntry>,
}

/// Devise d'affichage et taux appliqué, pour que l'interface écrive les
/// montants dans la monnaie choisie sans jamais recalculer de conversion.
#[derive(Debug, Clone, Serialize)]
pub struct CurrencyView {
    pub code: String,
    /// Combien vaut un dollar dans cette devise.
    pub rate: f64,
    /// Jour du taux, vide tant que rien n'a été relevé.
    pub day: String,
}

impl Default for CurrencyView {
    fn default() -> Self {
        CurrencyView {
            code: "USD".into(),
            rate: 1.0,
            day: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Overview {
    pub kpis: Kpis,
    /// Bornes effectives de la fenêtre affichée, toutes deux incluses.
    /// Le front les reprend telles quelles : il n'a jamais à recalculer de date.
    pub from: String,
    pub to: String,
    /// Mois `YYYY-MM` pour lesquels la base contient au moins une mesure.
    /// Alimente le filtre par mois, indépendamment de la fenêtre affichée.
    pub available_months: Vec<String>,
    /// Axe de jours dense couvrant toute la fenêtre, trous compris.
    /// Toutes les séries par projet sont alignées dessus.
    pub days: Vec<String>,
    pub timeline: Vec<TimelinePoint>,
    pub per_project: Vec<ProjectSummary>,
    pub countries: Vec<CountryTotal>,
    pub loaders: Vec<LoaderCell>,
    pub revenue: Vec<RevenuePoint>,
    pub revenue_by_project: Vec<RevenueByProject>,
    pub payout: Payout,
    pub events: Vec<EventRow>,
    pub freshness: Vec<Freshness>,
    pub curseforge_history_days: i64,
    pub curseforge_revenue: CfRevenue,
    pub currency: CurrencyView,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_roundtrips_through_str() {
        assert_eq!(Platform::from_str_lossy("modrinth"), Platform::Modrinth);
        assert_eq!(Platform::from_str_lossy("curseforge"), Platform::CurseForge);
        assert_eq!(Platform::Modrinth.as_str(), "modrinth");
        assert_eq!(Platform::CurseForge.as_str(), "curseforge");
    }
}
