use std::{collections::HashSet, fs, path::PathBuf, sync::mpsc};

use gtk4::glib;
use serde_json;
use walkdir::WalkDir;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub terminal: bool,
}

fn get_cache_path() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let cache_dir = PathBuf::from(home).join(".cache/hyprbucket");
        fs::create_dir_all(&cache_dir).ok()?;
        Some(cache_dir.join("desktop_entries.json"))
    } else {
        None
    }
}

fn load_cache(cache_path: &PathBuf) -> Option<Vec<DesktopEntry>> {
    let content = fs::read_to_string(cache_path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_cache(cache_path: &PathBuf, entries: &[DesktopEntry]) {
    if let Ok(json) = serde_json::to_string(entries) {
        let _ = fs::write(cache_path, json);
    }
}

pub fn load_desktop_entries() -> Vec<DesktopEntry> {
    if let Some(cache_path) = get_cache_path() {
        if let Some(cached) = load_cache(&cache_path) {
            return cached;
        }
    }

    load_desktop_entries_from_disk()
}

pub fn refresh_desktop_entries_async<F>(callback: F)
where
    F: Fn(Vec<DesktopEntry>) + 'static,
{
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let new_entries = load_desktop_entries_from_disk();

        let old_entries = if let Some(cache_path) = get_cache_path() {
            load_cache(&cache_path).unwrap_or_default()
        } else {
            Vec::new()
        };

        let old_ids: HashSet<_> = old_entries.iter().map(|e| e.id.clone()).collect();
        let newly_discovered: Vec<_> = new_entries
            .iter()
            .filter(|entry| !old_ids.contains(&entry.id))
            .cloned()
            .collect();

        if let Some(cache_path) = get_cache_path() {
            save_cache(&cache_path, &new_entries);
        }

        let _ = tx.send(newly_discovered);
    });

    glib::MainContext::default().spawn_local(async move {
        if let Ok(newly_discovered) = rx.recv() {
            if !newly_discovered.is_empty() {
                callback(newly_discovered);
            }
        }
    });
}

fn load_desktop_entries_from_disk() -> Vec<DesktopEntry> {
    let mut search_dirs = Vec::new();

    if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
        search_dirs.push(PathBuf::from(format!("{}/applications", xdg_data_home)));
    } else if let Ok(home) = std::env::var("HOME") {
        search_dirs.push(PathBuf::from(format!("{}/.local/share/applications", home)));
    }

    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg_data_dirs.split(':') {
            if !dir.is_empty() {
                search_dirs.push(PathBuf::from(format!("{}/applications", dir)));
            }
        }
    } else {
        search_dirs.push(PathBuf::from("/usr/local/share/applications"));
        search_dirs.push(PathBuf::from("/usr/share/applications"));
    }

    let mut seen_ids = HashSet::new();
    let mut entries = Vec::new();

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(&dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.extension().and_then(|ext| ext.to_str()) != Some("desktop") {
                continue;
            }

            if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                if seen_ids.insert(file_name.to_string()) {
                    if let Some(entry) = parse_desktop_file(path) {
                        entries.push(entry);
                    }
                }
            }
        }
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    entries
}

fn parse_desktop_file(path: &std::path::Path) -> Option<DesktopEntry> {
    let content = fs::read_to_string(path).ok()?;

    let mut name: Option<String> = None;
    let mut exec: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut terminal = false;
    let mut no_display = false;

    for line in content.lines() {
        if line.starts_with('[') || line.trim().is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();

        match key {
            "Name" if !key.contains('[') => {
                if name.is_none() {
                    name = Some(value.to_string());
                }
            }
            "Exec" if !key.contains('[') => {
                if exec.is_none() {
                    exec = Some(value.to_string());
                }
            }
            "Icon" if !key.contains('[') => {
                if icon.is_none() {
                    icon = Some(value.to_string());
                }
            }
            "Terminal" => {
                terminal = value == "true";
            }
            "NoDisplay" => {
                no_display = value == "true";
            }
            _ => {}
        }
    }

    if no_display || name.is_none() || exec.is_none() {
        return None;
    }

    let id = path.file_name()?.to_str()?.to_string();

    Some(DesktopEntry {
        id,
        name: name.unwrap(),
        exec: exec.unwrap(),
        icon,
        terminal,
    })
}
