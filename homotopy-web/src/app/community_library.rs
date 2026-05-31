use serde::Deserialize;

const COMMUNITY_INDEX: &str = include_str!("../../../community-library/generated/index.json");

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommunityPreset {
    pub id: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub min_app_version: String,
    pub tags: Vec<String>,
    pub axioms: Vec<String>,
    pub constructed: Vec<String>,
    pub source: String,
    pub didactic: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CommunityIndex {
    presets: Vec<CommunityPreset>,
}

pub fn presets() -> &'static [CommunityPreset] {
    use std::sync::OnceLock;

    static PRESETS: OnceLock<Vec<CommunityPreset>> = OnceLock::new();
    PRESETS
        .get_or_init(|| {
            serde_json::from_str::<CommunityIndex>(COMMUNITY_INDEX)
                .map(|index| index.presets)
                .unwrap_or_default()
        })
        .as_slice()
}

pub fn get(id: &str) -> Option<&'static CommunityPreset> {
    presets().iter().find(|preset| preset.id == id)
}

pub fn active_id(id: &str) -> String {
    format!("community:{id}")
}

pub fn parse_active_id(id: &str) -> Option<&str> {
    id.strip_prefix("community:")
}
