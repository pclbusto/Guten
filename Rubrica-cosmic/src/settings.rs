use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub sidebar_width: f32,
    pub recent_projects: Vec<PathBuf>,
    pub editor_wrap_text: bool,
    pub editor_font_size: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            sidebar_width: 280.0,
            recent_projects: Vec::new(),
            editor_wrap_text: true,
            editor_font_size: 14.0,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let path = Self::settings_path();
        if path.exists() {
            if let Ok(json) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str(&json) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::settings_path();
        fs::create_dir_all(path.parent().unwrap())?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn remember_recent_project(&mut self, path: &std::path::Path) {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.recent_projects.retain(|recent| recent != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(9);
    }

    fn settings_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gutenair")
            .join("rubrica-cosmic-settings.json")
    }
}
