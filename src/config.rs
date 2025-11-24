use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub pinned: Vec<String>,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();

        if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::create_default()
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        let content = toml::to_string(self).unwrap();
        fs::write(path, content).unwrap();
    }

    pub fn toggle_pin(&mut self, app_id: &str) {
        if let Some(pos) = self.pinned.iter().position(|id| id == app_id) {
            self.pinned.remove(pos);
        } else {
            self.pinned.push(app_id.to_string());
        }
        self.save();
    }

    fn config_path() -> PathBuf {
        let config_dir = PathBuf::from(std::env::var("HOME").unwrap())
            .join(".config")
            .join("hyprbucket");

        fs::create_dir_all(&config_dir).ok();
        config_dir.join("config.toml")
    }

    fn create_default() -> Self {
        let config = Self::default();
        config.save();
        config
    }
}
