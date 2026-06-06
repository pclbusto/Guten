use anyhow::{Context, Result};
use gutencore::{BookMetadata, GutenCore, TocEntry};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Posición de lectura persistente por libro
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReadingPosition {
    pub spine_index: usize,
    /// Offset vertical dentro del capítulo (píxeles) o porcentaje
    pub scroll_y: f64,
}

/// Modelo que encapsula el core y la navegación del lector
pub struct DocumentModel {
    pub core: GutenCore,
    pub spine_index: usize,
    pub config_dir: PathBuf,
    /// Cache de contenido HTML por spine ID (para paginación de dos páginas)
    pub html_cache: HashMap<String, String>,
}

impl DocumentModel {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let (core, _persistent_dir) = if path.is_dir() {
            (
                GutenCore::open_folder(path).context("Error abriendo carpeta EPUB")?,
                None,
            )
        } else {
            let persistent = Self::ensure_persistent_extraction(path)?;
            let core =
                GutenCore::open_folder(&persistent).context("Error abriendo EPUB extraído")?;
            (core, Some(persistent))
        };

        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gutenreader");
        fs::create_dir_all(&config_dir)?;

        let mut model = Self {
            core,
            spine_index: 0,
            config_dir,
            html_cache: HashMap::new(),
        };

        // Intentar restaurar posición
        let _ = model.load_position();
        Ok(model)
    }

    fn ensure_persistent_extraction(epub_path: &std::path::Path) -> Result<PathBuf> {
        let cache_base = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gutenreader")
            .join("epubs");
        fs::create_dir_all(&cache_base)?;

        // Calcular hash del archivo para usar como nombre de carpeta
        let bytes = fs::read(epub_path)?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let target = cache_base.join(&hash);

        if target.join("META-INF").exists() {
            eprintln!(
                "[GutenReader] Usando extracción persistente en {:?}",
                target
            );
            return Ok(target);
        }

        eprintln!("[GutenReader] Extrayendo EPUB a {:?}", target);
        fs::create_dir_all(&target)?;
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;
        archive.extract(&target)?;

        Ok(target)
    }

    pub fn book_id(&self) -> String {
        if let Some(meta) = &self.core.metadata {
            meta.identifier.clone()
        } else if let Some(orig) = &self.core.original_epub {
            format!("{:?}", orig)
        } else {
            format!("{:?}", self.core.workdir)
        }
    }

    pub fn current_chapter_id(&self) -> Option<&str> {
        self.core.spine.get(self.spine_index).map(|s| s.as_str())
    }

    pub fn current_chapter_html(&mut self) -> Result<String> {
        let id = self
            .current_chapter_id()
            .context("No hay capítulo actual")?;

        if let Some(html) = self.html_cache.get(id) {
            return Ok(html.clone());
        }

        let path = self
            .core
            .get_resource_path(id)
            .with_context(|| format!("No se encontró el recurso {}", id))?;

        let content =
            fs::read_to_string(&path).with_context(|| format!("Error leyendo capítulo {}", id))?;

        self.html_cache.insert(id.to_string(), content.clone());
        Ok(content)
    }

    pub fn chapter_html_by_id(&mut self, id: &str) -> Result<String> {
        if let Some(html) = self.html_cache.get(id) {
            return Ok(html.clone());
        }
        let path = self
            .core
            .get_resource_path(id)
            .with_context(|| format!("No se encontró el recurso {}", id))?;
        let content = fs::read_to_string(&path)?;
        self.html_cache.insert(id.to_string(), content.clone());
        Ok(content)
    }

    pub fn goto_next(&mut self) -> bool {
        if self.spine_index + 1 < self.core.spine.len() {
            self.spine_index += 1;
            true
        } else {
            false
        }
    }

    pub fn goto_prev(&mut self) -> bool {
        if self.spine_index > 0 {
            self.spine_index -= 1;
            true
        } else {
            false
        }
    }

    pub fn goto_spine_index(&mut self, index: usize) -> bool {
        if index < self.core.spine.len() {
            self.spine_index = index;
            true
        } else {
            false
        }
    }

    /// Navega a un href del TOC, que puede ser "Text/chap.xhtml#anchor"
    pub fn goto_toc_href(&mut self, href: &str) -> bool {
        let (file_part, anchor) = if let Some(pos) = href.find('#') {
            (&href[..pos], Some(&href[pos + 1..]))
        } else {
            (href, None)
        };

        let decoded_file_part = percent_decode(file_part);
        let target_path = self.resolve_toc_href_path(&decoded_file_part);
        let target_file_name = Path::new(&decoded_file_part)
            .file_name()
            .map(|name| name.to_os_string());
        let href_has_dir = Path::new(&decoded_file_part)
            .parent()
            .is_some_and(|parent| {
                parent
                    .components()
                    .any(|component| !matches!(component, Component::CurDir))
            });

        if let Some((idx, _)) = self.core.spine.iter().enumerate().find(|(_, id)| {
            self.core
                .get_resource_path(id)
                .map(|path| {
                    paths_equivalent(&path, &target_path)
                        || (!href_has_dir
                            && target_file_name
                                .as_ref()
                                .is_some_and(|name| path.file_name() == Some(name.as_os_str())))
                })
                .unwrap_or(false)
        }) {
            self.spine_index = idx;
            // TODO: scroll to anchor (se hará vía JS en el WebView)
            let _ = anchor;
            true
        } else {
            false
        }
    }

    fn resolve_toc_href_path(&self, file_part: &str) -> PathBuf {
        let path = Path::new(file_part.trim_start_matches('/'));
        let base = self.core.opf_dir.as_ref().unwrap_or(&self.core.workdir);

        normalize_path(&base.join(path))
    }

    pub fn goto_chapter_id(&mut self, id: &str) -> bool {
        if let Some((idx, _)) = self
            .core
            .spine
            .iter()
            .enumerate()
            .find(|(_, sid)| *sid == id)
        {
            self.spine_index = idx;
            true
        } else {
            false
        }
    }

    /// Busca el primer capítulo con texto sustancial (>3000 chars).
    /// Saltea portadas, prefacios y copyright.
    pub fn find_first_content_chapter(&mut self) -> usize {
        let ids: Vec<String> = self.core.spine.clone();
        for (idx, id) in ids.iter().enumerate() {
            if let Ok(html) = self.chapter_html_by_id(id) {
                let text = Self::quick_plaintext(&html);
                if text.len() > 3000 {
                    eprintln!(
                        "[GutenReader] Primer capítulo largo: spine[{}] con {} chars",
                        idx,
                        text.len()
                    );
                    return idx;
                }
            }
        }
        0
    }

    fn quick_plaintext(html: &str) -> String {
        let mut text = String::new();
        let mut in_tag = false;
        for ch in html.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => {
                    in_tag = false;
                    text.push(' ');
                }
                _ if !in_tag => text.push(ch),
                _ => {}
            }
        }
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    pub fn spine_len(&self) -> usize {
        self.core.spine.len()
    }

    pub fn toc(&self) -> Result<Vec<TocEntry>> {
        self.core.get_toc().context("Error obteniendo TOC")
    }

    pub fn metadata(&self) -> Option<&BookMetadata> {
        self.core.get_metadata()
    }

    pub fn base_uri(&self) -> Option<String> {
        let id = self.current_chapter_id()?;
        let path = self.core.get_resource_path(id).ok()?;
        let parent = path.parent()?;
        let canon = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        // Convertir a URI válida (espacios -> %20, etc.)
        gtk::glib::filename_to_uri(canon, None).ok().map(|uri| {
            let mut s = uri.to_string();
            if !s.ends_with('/') {
                s.push('/');
            }
            s
        })
    }

    pub fn save_position(&self, scroll_y: f64) -> Result<()> {
        let pos = ReadingPosition {
            spine_index: self.spine_index,
            scroll_y,
        };
        let path = self.position_file();
        let json = serde_json::to_string_pretty(&pos)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_position(&mut self) -> Result<()> {
        let path = self.position_file();
        if path.exists() {
            let json = fs::read_to_string(path)?;
            let pos: ReadingPosition = serde_json::from_str(&json)?;
            if pos.spine_index < self.core.spine.len() {
                self.spine_index = pos.spine_index;
            }
        }
        Ok(())
    }

    fn position_file(&self) -> PathBuf {
        let id = sanitize_id(&self.book_id());
        self.config_dir.join(format!("pos_{}.json", id))
    }

    pub fn search(&self, query: &str) -> Result<Vec<gutencore::SearchResult>> {
        self.core.search(query).context("Error en búsqueda")
    }
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn paths_equivalent(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| normalize_path(left));
    let right = right
        .canonicalize()
        .unwrap_or_else(|_| normalize_path(right));
    left == right
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                decoded.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
