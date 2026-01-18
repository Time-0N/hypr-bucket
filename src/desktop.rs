use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    thread,
};

use async_channel::Sender;
use walkdir::WalkDir;

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub terminal: bool,
}

pub enum LoaderMsg {
    Batch(Vec<DesktopEntry>),
    App(DesktopEntry),
    Remove(Vec<String>),
    Done,
}

fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(xdg_data_home).join("applications"));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg_data_dirs.split(':').filter(|s| !s.is_empty()) {
            dirs.push(PathBuf::from(dir).join("applications"));
        }
    } else {
        dirs.push(PathBuf::from("/usr/local/share/applications"));
        dirs.push(PathBuf::from("/usr/share/applications"));
    }

    dirs
}

pub fn spawn_load_entries(sender: Sender<LoaderMsg>) {
    thread::spawn(move || {
        let mut cached_map: HashMap<String, DesktopEntry> = HashMap::new();
        let mut cached_ids = HashSet::new();
        let mut found_ids = HashSet::new();
        let mut fresh_list = Vec::new();

        if let Some(cache_path) = get_cache_path() {
            if let Some(cached_apps) = load_cache(&cache_path) {
                let mut unique = Vec::new();
                for app in cached_apps {
                    if cached_map.contains_key(&app.id) {
                        continue;
                    }
                    cached_ids.insert(app.id.clone());
                    cached_map.insert(app.id.clone(), app.clone());
                    unique.push(app);
                }
                let _ = sender.send_blocking(LoaderMsg::Batch(unique));
            }
        }

        for dir in application_dirs() {
            if !dir.exists() {
                continue;
            }

            for entry in WalkDir::new(dir)
                .follow_links(true)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("desktop") {
                    continue;
                }

                if let Some(desktop_entry) = parse_desktop_file(path) {
                    let id = desktop_entry.id.clone();

                    if !found_ids.insert(id.clone()) {
                        continue;
                    }

                    fresh_list.push(desktop_entry.clone());

                    let changed = cached_map
                        .get(&id)
                        .map(|c| c != &desktop_entry)
                        .unwrap_or(true);

                    if changed {
                        let _ = sender.send_blocking(LoaderMsg::App(desktop_entry));
                    }
                }
            }
        }

        let to_remove: Vec<String> = cached_ids.difference(&found_ids).cloned().collect();
        if !to_remove.is_empty() {
            let _ = sender.send_blocking(LoaderMsg::Remove(to_remove));
        }

        fresh_list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        if let Some(cache_path) = get_cache_path() {
            save_cache(&cache_path, &fresh_list);
        }

        let _ = sender.send_blocking(LoaderMsg::Done);
    });
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
