use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::desktop::DesktopEntry;

pub fn search_apps(query: &str, apps: &[DesktopEntry]) -> Vec<DesktopEntry> {
    if query.is_empty() {
        return apps.to_vec();
    }

    let matcher = SkimMatcherV2::default().ignore_case();

    let mut results: Vec<_> = apps
        .iter()
        .filter_map(|app| {
            matcher
                .fuzzy_match(&app.name, query)
                .map(|score| (score, app.clone()))
        })
        .collect();

    results.sort_by(|a, b| match b.0.cmp(&a.0) {
        std::cmp::Ordering::Equal => a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()),
        other => other,
    });

    results.into_iter().map(|(_, app)| app).collect()
}
