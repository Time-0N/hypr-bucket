use std::{collections::HashMap, fs, path::PathBuf};

use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub terminal: bool,
}

pub fn load_desktop_entries() -> Vec<DesktopEntry> {
    let mut entries = Vec::new();

    let search_dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from(format!(
            "{}/.local/share/applications",
            std::env::var("HOME").unwrap_or_default()
        )),
    ];

    search_dirs
        .into_iter()
        .filter(|dir| dir.exists())
        .flat_map(|dir| {
            WalkDir::new(dir)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
        })
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("desktop"))
        .filter_map(|path| parse_desktop_file(&path))
        .collect()
}

fn parse_desktop_file(path: &std::path::Path) -> Option<DesktopEntry> {
    let content = fs::read_to_string(path).ok()?;

    let props: HashMap<String, String> = content
        .lines()
        .filter(|line| !line.starts_with('[') && !line.trim().is_empty())
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect();

    if props.get("NoDisplay").map(|v| v == "true").unwrap_or(false) {
        return None;
    }

    let name = props.get("Name")?.clone();
    let exec = props.get("Exec")?.clone();
    let icon = props.get("Icon").cloned();
    let terminal = props.get("Terminal").map(|v| v == "true").unwrap_or(false);

    let id = path.file_name()?.to_str()?.to_string();

    Some(DesktopEntry {
        id,
        name,
        exec,
        icon,
        terminal,
    })
}
