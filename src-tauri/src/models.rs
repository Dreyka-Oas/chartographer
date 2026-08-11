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
    pub downloads_prev_30d: i64,
    pub revenue_total: String,
    pub revenue_pending: String,
    pub followers: i64,
    pub projects_active: i64,
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
pub struct Overview {
    pub kpis: Kpis,
    pub timeline: Vec<TimelinePoint>,
    pub per_project: Vec<ProjectSummary>,
    pub countries: Vec<CountryTotal>,
    pub loaders: Vec<LoaderCell>,
    pub revenue: Vec<RevenuePoint>,
    pub events: Vec<EventRow>,
    pub freshness: Vec<Freshness>,
    pub curseforge_history_days: i64,
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
