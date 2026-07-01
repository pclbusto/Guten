use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub chapter_id: String,
    pub selected_text: String,
    pub note: String,
    pub color: String,
    pub created_at: String,
    pub anchor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnnotationStore {
    pub annotations: Vec<Annotation>,
    book_hash: String,
}

#[allow(dead_code)]
impl AnnotationStore {
    pub fn new(book_hash: &str) -> Self {
        let path = Self::store_path(book_hash);
        if path.exists() {
            if let Ok(json) = fs::read_to_string(&path) {
                if let Ok(store) = serde_json::from_str::<Self>(&json) {
                    if store.book_hash == book_hash {
                        return store;
                    }
                }
            }
        }
        Self {
            annotations: Vec::new(),
            book_hash: book_hash.to_string(),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::store_path(&self.book_hash);
        fs::create_dir_all(path.parent().unwrap())?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn add(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }

    pub fn for_chapter(&self, chapter_id: &str) -> Vec<&Annotation> {
        self.annotations
            .iter()
            .filter(|a| a.chapter_id == chapter_id)
            .collect()
    }

    fn store_path(book_hash: &str) -> PathBuf {
        let safe = book_hash
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gutenreader")
            .join("annotations")
            .join(format!("{}.json", safe))
    }
}
