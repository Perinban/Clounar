use serde::Deserialize;
use uuid::Uuid;

use crate::constants::{
    SEARCH_FOCUS_INTERNET, SEARCH_FOCUS_WRITING, SEARCH_SOURCE_NONE, SEARCH_SOURCE_WEB,
    SUPPORTED_BLOCK_USE_CASES, SUPPORTED_FEATURES,
};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Web,
    Writing,
    Strict,
}

pub struct SearchProfile {
    pub search_focus: &'static str,
    pub source: &'static str,
    pub sources: &'static [&'static str],
    pub skip_search_enabled: bool,
    pub always_search_override: bool,
    pub expect_search_results: bool,
    pub supported_block_use_cases: &'static [&'static str],
    pub supported_features: &'static [&'static str],
}

pub struct SearchParams<'a> {
    pub query: &'a str,
    pub mode: &'a str,
    pub model: &'a str,
    pub incognito: bool,
    pub search_mode: &'a SearchMode,
    pub context_uuid: Option<Uuid>,
}

impl SearchMode {
    pub fn profile(&self) -> SearchProfile {
        match self {
            SearchMode::Web => SearchProfile {
                search_focus: SEARCH_FOCUS_INTERNET,
                source: SEARCH_SOURCE_WEB,
                sources: &[SEARCH_SOURCE_WEB],
                skip_search_enabled: false,
                always_search_override: true,
                expect_search_results: true,
                supported_block_use_cases: SUPPORTED_BLOCK_USE_CASES,
                supported_features: SUPPORTED_FEATURES,
            },
            SearchMode::Writing => SearchProfile {
                search_focus: SEARCH_FOCUS_WRITING,
                source: SEARCH_SOURCE_NONE,
                sources: &[SEARCH_SOURCE_NONE],
                skip_search_enabled: true,
                always_search_override: false,
                expect_search_results: false,
                supported_block_use_cases: SUPPORTED_BLOCK_USE_CASES,
                supported_features: SUPPORTED_FEATURES,
            },
            SearchMode::Strict => SearchProfile {
                search_focus: SEARCH_FOCUS_WRITING,
                source: SEARCH_SOURCE_NONE,
                sources: &[SEARCH_SOURCE_NONE],
                skip_search_enabled: true,
                always_search_override: false,
                expect_search_results: false,
                supported_block_use_cases: SUPPORTED_BLOCK_USE_CASES,
                supported_features: SUPPORTED_FEATURES,
            },
        }
    }
}
