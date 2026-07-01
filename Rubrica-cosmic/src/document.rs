use anyhow::{Context, Result};
use gutencore::{BookMetadata, GutenCore, ManifestItem, ResourceKind};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ProjectModel {
    pub core: GutenCore,
    pub original_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ResourceGroup {
    pub kind: ResourceKind,
    pub label: &'static str,
    pub items: Vec<ManifestItem>,
}

impl ProjectModel {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let core = if path.is_dir() {
            GutenCore::open_folder(path).context("Error abriendo carpeta de proyecto")?
        } else {
            GutenCore::open_epub(path).context("Error abriendo archivo EPUB")?
        };

        Ok(Self {
            core,
            original_path: path.to_path_buf(),
        })
    }

    pub fn create_new<P: AsRef<Path>>(root: P, title: &str, lang: &str) -> Result<Self> {
        let core = GutenCore::new_project(root.as_ref(), title, lang)
            .context("Error creando proyecto nuevo")?;
        Ok(Self {
            core,
            original_path: root.as_ref().to_path_buf(),
        })
    }

    #[allow(dead_code)]
    pub fn book_id(&self) -> String {
        if let Some(meta) = &self.core.metadata {
            meta.identifier.clone()
        } else if let Some(orig) = &self.core.original_epub {
            format!("{:?}", orig)
        } else {
            format!("{:?}", self.core.workdir)
        }
    }

    pub fn title(&self) -> String {
        self.core
            .metadata
            .as_ref()
            .map(|m| m.title.clone())
            .unwrap_or_else(|| {
                self.original_path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Proyecto".into())
            })
    }

    #[allow(dead_code)]
    pub fn metadata(&self) -> Option<&BookMetadata> {
        self.core.get_metadata()
    }

    pub fn resource_kind(media_type: &str) -> ResourceKind {
        match media_type {
            "application/xhtml+xml" | "text/html" => ResourceKind::Document,
            "text/css" => ResourceKind::Style,
            mt if mt.starts_with("image/") => ResourceKind::Image,
            mt if mt.starts_with("font/")
                || mt == "application/font-sfnt"
                || mt == "application/font-woff"
                || mt == "application/vnd.ms-opentype" =>
            {
                ResourceKind::Font
            }
            mt if mt.starts_with("audio/") => ResourceKind::Audio,
            mt if mt.starts_with("video/") => ResourceKind::Video,
            "application/javascript" | "text/javascript" => ResourceKind::Script,
            "image/svg+xml" => ResourceKind::Vector,
            mt if mt.contains("navigation") || mt.contains("nav") => ResourceKind::Navigation,
            _ => ResourceKind::Other,
        }
    }

    pub fn grouped_resources(&self) -> Vec<ResourceGroup> {
        let order = [
            ResourceKind::Document,
            ResourceKind::Style,
            ResourceKind::Image,
            ResourceKind::Font,
            ResourceKind::Audio,
            ResourceKind::Video,
            ResourceKind::Vector,
            ResourceKind::Script,
            ResourceKind::Navigation,
            ResourceKind::Other,
        ];

        let mut groups: Vec<ResourceGroup> = order
            .iter()
            .map(|kind| ResourceGroup {
                kind: *kind,
                label: label_for_kind(*kind),
                items: Vec::new(),
            })
            .collect();

        for item in self.core.manifest.values() {
            let kind = Self::resource_kind(&item.media_type);
            if let Some(group) = groups.iter_mut().find(|g| g.kind == kind) {
                group.items.push(item.clone());
            }
        }

        for group in &mut groups {
            group.items.sort_by(|a, b| a.href.cmp(&b.href));
        }

        groups.into_iter().filter(|g| !g.items.is_empty()).collect()
    }

    pub fn is_text_editable(media_type: &str) -> bool {
        matches!(
            media_type,
            "application/xhtml+xml"
                | "text/html"
                | "text/css"
                | "text/xml"
                | "application/xml"
                | "text/plain"
                | "application/javascript"
                | "text/javascript"
        )
    }

    pub fn load_resource_text(&self, id: &str) -> Result<String> {
        let path = self
            .core
            .get_resource_path(id)
            .with_context(|| format!("No se encontró el recurso {}", id))?;
        fs::read_to_string(&path).with_context(|| format!("Error leyendo {}", path.display()))
    }

    pub fn save_resource_text(&mut self, id: &str, content: &str) -> Result<()> {
        let item = self
            .core
            .manifest
            .get(id)
            .with_context(|| format!("El recurso {} no existe en el manifiesto", id))?;

        if Self::is_text_editable(&item.media_type) {
            if item.media_type == "application/xhtml+xml" {
                self.core
                    .save_chapter(id, content)
                    .with_context(|| format!("Error guardando capítulo {}", id))?;
            } else {
                let path = self
                    .core
                    .get_resource_path(id)
                    .with_context(|| format!("No se encontró el recurso {}", id))?;
                fs::write(&path, content)
                    .with_context(|| format!("Error escribiendo {}", path.display()))?;
            }
        } else {
            anyhow::bail!("El recurso {} no es editable como texto", id);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn resource_path(&self, id: &str) -> Result<PathBuf> {
        Ok(self.core.get_resource_path(id)?)
    }
}

fn label_for_kind(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Document => "Documentos",
        ResourceKind::Style => "Estilos",
        ResourceKind::Image => "Imágenes",
        ResourceKind::Font => "Fuentes",
        ResourceKind::Audio => "Audio",
        ResourceKind::Video => "Video",
        ResourceKind::Vector => "Vector",
        ResourceKind::Script => "Scripts",
        ResourceKind::Navigation => "Navegación",
        ResourceKind::Other => "Otros",
    }
}
