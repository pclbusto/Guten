use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_READER_FONT_SIZE_PT: f64 = 14.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IlluminationProfile {
    pub name: String,
    pub text_color: String,
    pub bg_color: String,
}

impl Default for IlluminationProfile {
    fn default() -> Self {
        Self {
            name: "Día".to_string(),
            text_color: "#1a1a1a".to_string(),
            bg_color: "#faf8f5".to_string(),
        }
    }
}

impl IlluminationProfile {
    pub fn night() -> Self {
        Self {
            name: "Noche".to_string(),
            text_color: "#d8d8d8".to_string(),
            bg_color: "#1a1a1a".to_string(),
        }
    }

    pub fn sepia() -> Self {
        Self {
            name: "Sepia".to_string(),
            text_color: "#5b4636".to_string(),
            bg_color: "#f4ecd8".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderSettings {
    pub font_family: String,
    pub font_size_pt: f64,
    pub line_height: f64,
    pub margin_em: f64,
    pub text_align: String,
    pub current_profile: String,
    pub profiles: HashMap<String, IlluminationProfile>,
    pub tts_voice: String,
    #[serde(default)]
    pub recent_books: Vec<PathBuf>,
}

impl Default for ReaderSettings {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert("day".to_string(), IlluminationProfile::default());
        profiles.insert("night".to_string(), IlluminationProfile::night());
        profiles.insert("sepia".to_string(), IlluminationProfile::sepia());

        Self {
            font_family: "Sans".to_string(),
            font_size_pt: DEFAULT_READER_FONT_SIZE_PT,
            line_height: 1.6,
            margin_em: 2.0,
            text_align: "left".to_string(),
            current_profile: "day".to_string(),
            profiles,
            tts_voice: String::new(),
            recent_books: Vec::new(),
        }
    }
}

impl ReaderSettings {
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

    pub fn current_profile(&self) -> &IlluminationProfile {
        self.profiles
            .get(&self.current_profile)
            .unwrap_or_else(|| self.profiles.get("day").unwrap())
    }

    pub fn remember_recent_book(&mut self, path: &std::path::Path) {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.recent_books.retain(|recent| recent != &path);
        self.recent_books.insert(0, path);
        self.recent_books.truncate(9);
    }

    pub fn recent_book(&self, index: usize) -> Option<PathBuf> {
        self.recent_books.get(index).cloned()
    }

    fn settings_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gutenreader")
            .join("settings.json")
    }
}

#[cfg(test)]
mod tests {
    use super::ReaderSettings;
    use std::path::{Path, PathBuf};

    #[test]
    fn recent_books_are_deduplicated_and_limited_to_nine() {
        let mut settings = ReaderSettings::default();
        for index in 0..12 {
            settings.remember_recent_book(Path::new(&format!("missing-book-{index}.epub")));
        }

        assert_eq!(settings.recent_books.len(), 9);
        assert_eq!(
            settings.recent_book(0),
            Some(PathBuf::from("missing-book-11.epub"))
        );

        settings.remember_recent_book(Path::new("missing-book-5.epub"));
        assert_eq!(settings.recent_books.len(), 9);
        assert_eq!(
            settings.recent_book(0),
            Some(PathBuf::from("missing-book-5.epub"))
        );
        assert_eq!(
            settings
                .recent_books
                .iter()
                .filter(|path| *path == Path::new("missing-book-5.epub"))
                .count(),
            1
        );
    }
}
