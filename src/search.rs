use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

use crate::desktop::DesktopEntry;

pub fn search_apps(query: &str, apps: &[DesktopEntry]) -> Vec<DesktopEntry> {
    if query.is_empty() {
        return apps.to_vec();
    }

    let matcher = SkimMatcherV2::default();
    let mut results: Vec<_> = apps
        .iter()
        .filter_map(|app| {
            matcher
                .fuzzy_match(&app.name, query)
                .map(|score| (score, app.clone()))
        })
        .collect();

    results.sort_by(|a, b| b.0.cmp(&a.0));
    results.into_iter().map(|(_, app)| app).collect();
}
