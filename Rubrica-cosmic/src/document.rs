use anyhow::{Context, Result};
use gutencore::{BookMetadata, DocToc, GutenCore, ManifestItem, ResourceKind};
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
            if group.kind == ResourceKind::Document {
                group.items.sort_by(|a, b| {
                    let pos_a = self.core.spine.iter().position(|id| id == &a.id);
                    let pos_b = self.core.spine.iter().position(|id| id == &b.id);
                    match (pos_a, pos_b) {
                        (Some(pa), Some(pb)) => pa.cmp(&pb),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.href.cmp(&b.href),
                    }
                });
            } else {
                group.items.sort_by(|a, b| a.href.cmp(&b.href));
            }
        }

        groups.into_iter().filter(|g| !g.items.is_empty()).collect()
    }

    pub fn unique_id(&self, prefix: &str) -> String {
        let mut n = 1;
        loop {
            let id = format!("{}{}", prefix, n);
            if !self.core.manifest.contains_key(&id) {
                return id;
            }
            n += 1;
        }
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

    /// Persiste el proyecto completo. Los EPUB abiertos se reconstruyen y
    /// reemplazan en su ruta original; los proyectos de carpeta guardan su OPF.
    pub fn save(&mut self) -> Result<()> {
        if self.core.original_epub.is_some() {
            self.core
                .save_epub()
                .context("Error reconstruyendo el archivo EPUB")
        } else {
            self.core.save().context("Error guardando el proyecto")
        }
    }

    pub fn create_chapter(&mut self, id: &str, title: &str) -> Result<()> {
        let href = format!("Text/{}.xhtml", id);
        self.core
            .add_to_manifest(
                id.to_string(),
                href,
                "application/xhtml+xml".to_string(),
                "".to_string(),
            )
            .with_context(|| format!("Error registrando capítulo {}", id))?;

        let head_links = if let Some(style) = self.core.config.default_styles.first() {
            if let Some(item) = self.core.manifest.get(style) {
                format!(
                    "\n  <link rel=\"stylesheet\" type=\"text/css\" href=\"../{}\"/>",
                    item.href
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let body = format!("  <h1>{}</h1>\n  <p></p>", title);
        let lang = self
            .core
            .metadata
            .as_ref()
            .map(|m| m.language.as_str())
            .unwrap_or("es");
        let xhtml = GutenCore::build_xhtml(lang, title, &head_links, &body);

        self.core
            .save_chapter(id, &xhtml)
            .with_context(|| format!("Error creando capítulo {}", id))?;

        if !self.core.spine.contains(&id.to_string()) {
            self.core.spine.push(id.to_string());
        }

        self.core
            .save()
            .with_context(|| "Error guardando el proyecto después de crear el capítulo")?;
        self.core
            .build_index()
            .with_context(|| "Error reconstruyendo el índice")?;
        Ok(())
    }

    pub fn create_style(&mut self, id: &str) -> Result<()> {
        let css = "/* Nuevo estilo */\n";
        self.core
            .add_style(id, css)
            .with_context(|| format!("Error creando estilo {}", id))?;
        self.core
            .save()
            .with_context(|| "Error guardando el proyecto después de crear el estilo")?;
        Ok(())
    }

    pub fn import_image(&mut self, source_path: &Path) -> Result<String> {
        let file_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.png");
        let id = Self::sanitize_id(file_name.rsplitn(2, '.').last().unwrap_or(file_name));
        let href = format!("Images/{}", file_name);
        let mime_type = guess_image_mime(file_name);

        self.core
            .import_file(source_path, id.clone(), &href, mime_type)
            .with_context(|| format!("Error importando imagen {}", source_path.display()))?;
        self.core
            .save()
            .with_context(|| "Error guardando el proyecto después de importar la imagen")?;
        Ok(id)
    }

    pub fn import_font(&mut self, source_path: &Path) -> Result<String> {
        let file_name = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("font.ttf");
        let id = format!(
            "font-{}",
            Self::sanitize_id(file_name.rsplitn(2, '.').last().unwrap_or(file_name))
        );
        let href = format!("Fonts/{}", file_name);
        let mime_type = guess_font_mime(file_name);

        self.core
            .import_file(source_path, id.clone(), &href, mime_type)
            .with_context(|| format!("Error importando fuente {}", source_path.display()))?;
        self.core
            .save()
            .with_context(|| "Error guardando el proyecto después de importar la fuente")?;
        Ok(id)
    }

    pub fn set_cover_image(&mut self, id: &str) -> Result<()> {
        self.core
            .set_cover_from_resource(id)
            .with_context(|| format!("Error configurando la imagen {} como portada", id))?;
        self.core
            .save()
            .context("Error guardando el proyecto después de configurar la portada")?;
        Ok(())
    }

    pub fn rename_resource(&mut self, id: &str, new_href: &str) -> Result<()> {
        let mut renames = std::collections::HashMap::new();
        renames.insert(id.to_string(), new_href.to_string());
        self.core
            .rename_files(renames)
            .context("Error renombrando el recurso")?;
        self.core
            .save()
            .context("Error guardando el proyecto después de renombrar")?;
        Ok(())
    }

    fn sanitize_id(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .to_lowercase()
    }

    #[allow(dead_code)]
    pub fn resource_path(&self, id: &str) -> Result<PathBuf> {
        Ok(self.core.get_resource_path(id)?)
    }

    /// Escanea todos los documentos XHTML del spine y devuelve la información
    /// de encabezados para construir la tabla de contenidos.
    ///
    /// Si algún archivo falla (no existe, XML mal formado, etc.) se ignora y se
    /// sigue con el resto. El vector de errores permite informar al usuario qué
    /// archivos tuvieron problemas sin bloquear la generación de la TOC.
    pub fn get_toc_data(&self) -> Result<(Vec<DocToc>, Vec<String>)> {
        let mut data = Vec::new();
        let mut errors = Vec::new();

        for idref in &self.core.spine {
            let Some(item) = self.core.manifest.get(idref) else {
                continue;
            };
            if item.media_type != "application/xhtml+xml" {
                continue;
            }
            match self.core.scan_headings(&item.href) {
                Ok(mut doc_toc) => {
                    if doc_toc.items.is_empty() {
                        doc_toc.title = idref.clone();
                    } else {
                        doc_toc.title = doc_toc.items[0].title.clone();
                    }
                    data.push(doc_toc);
                }
                Err(e) => {
                    let msg = format!("{} ({}): {}", idref, item.href, e);
                    eprintln!("[get_toc_data] falló scan_headings: {}", msg);
                    errors.push(msg);
                }
            }
        }

        if data.is_empty() && !errors.is_empty() {
            anyhow::bail!(
                "No se pudo escanear ningún documento. Errores: {}",
                errors.join("; ")
            );
        }

        Ok((data, errors))
    }

    /// Reconstruye `nav.xhtml` (EPUB 3) y `toc.ncx` (EPUB 2) a partir de los
    /// datos de `DocToc` proporcionados, respetando las inclusiones del usuario.
    /// Si `create_ncx` es `false` y el proyecto no tiene `toc.ncx`, no se creará.
    pub fn build_navigation(&mut self, data: &[DocToc], create_ncx: bool) -> Result<()> {
        self.core
            .build_nav_from_data(data)
            .context("Error generando nav.xhtml")?;
        let has_ncx = self
            .core
            .manifest
            .values()
            .any(|it| it.media_type == "application/x-dtbncx+xml");
        if create_ncx || has_ncx {
            self.build_ncx_from_data(data)
                .context("Error generando toc.ncx")?;
        }
        self.core
            .save()
            .context("Error guardando el proyecto después de regenerar la navegación")?;
        Ok(())
    }

    /// Regenera la navegación usando todos los encabezados de los documentos
    /// actuales del spine. Solo crea `toc.ncx` si ya existe en el proyecto.
    pub fn rebuild_navigation_from_spine(&mut self) -> Result<()> {
        let (data, _errors) = self.get_toc_data()?;
        self.build_navigation(&data, false)
    }

    fn build_ncx_from_data(&mut self, data: &[DocToc]) -> Result<()> {
        let opf_dir = self
            .core
            .opf_dir
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("OPF dir no cargado"))?
            .clone();

        let title = self
            .core
            .metadata
            .as_ref()
            .map(|m| m.title.as_str())
            .unwrap_or("Índice");
        let identifier = self
            .core
            .metadata
            .as_ref()
            .map(|m| m.identifier.as_str())
            .unwrap_or("");

        // Si ya existe un item NCX en el manifiesto, usar su ruta; si no, usar toc.ncx.
        let ncx_item = self
            .core
            .manifest
            .values()
            .find(|it| it.media_type == "application/x-dtbncx+xml")
            .cloned();

        let (ncx_id, ncx_href) = if let Some(item) = ncx_item {
            (item.id.clone(), item.href.clone())
        } else {
            ("ncx".to_string(), "toc.ncx".to_string())
        };

        let ncx_path = opf_dir.join(&ncx_href);
        let ncx_parent = ncx_path.parent().unwrap_or_else(|| opf_dir.as_path());

        let mut entries: Vec<(u8, String, String)> = Vec::new();

        for doc in data {
            if !doc.include {
                continue;
            }
            let doc_path = std::path::Path::new(&doc.href);
            let rel = pathdiff::diff_paths(doc_path, ncx_parent)
                .unwrap_or_else(|| doc_path.to_path_buf());
            let rel_str = rel.to_string_lossy().replace('\\', "/");

            let has_visible_items = doc.items.iter().any(|h| h.include);
            if has_visible_items {
                for heading in &doc.items {
                    if !heading.include {
                        continue;
                    }
                    let href = if heading.anchor.is_empty() {
                        rel_str.clone()
                    } else {
                        format!("{}#{}", rel_str, heading.anchor)
                    };
                    entries.push((heading.level, heading.title.clone(), href));
                }
            } else {
                entries.push((1, doc.title.clone(), rel_str));
            }
        }

        let max_level = entries.iter().map(|e| e.0).max().unwrap_or(1);

        let mut ncx = String::new();
        ncx.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        ncx.push_str("<ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\">\n");
        ncx.push_str("  <head>\n");
        ncx.push_str(&format!(
            "    <meta name=\"dtb:uid\" content=\"{}\"/>\n",
            Self::escape_xml(identifier)
        ));
        ncx.push_str(&format!("    <meta name=\"dtb:depth\" content=\"{}\"/>\n", max_level));
        ncx.push_str("    <meta name=\"dtb:totalPageCount\" content=\"0\"/>\n");
        ncx.push_str("    <meta name=\"dtb:maxPageNumber\" content=\"0\"/>\n");
        ncx.push_str("  </head>\n");
        ncx.push_str("  <docTitle>\n");
        ncx.push_str(&format!(
            "    <text>{}</text>\n",
            Self::escape_xml(title)
        ));
        ncx.push_str("  </docTitle>\n");
        ncx.push_str("  <navMap>\n");

        for (idx, (_level, title, href)) in entries.iter().enumerate() {
            let play_order = idx + 1;
            ncx.push_str(&format!(
                "    <navPoint id=\"navPoint-{}\" playOrder=\"{}\">\n",
                play_order, play_order
            ));
            ncx.push_str(&format!(
                "      <navLabel><text>{}</text></navLabel>\n",
                Self::escape_xml(title)
            ));
            ncx.push_str(&format!(
                "      <content src=\"{}\"/>\n",
                Self::escape_xml(href)
            ));
            ncx.push_str("    </navPoint>\n");
        }

        ncx.push_str("  </navMap>\n");
        ncx.push_str("</ncx>\n");

        if let Some(parent) = ncx_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&ncx_path, ncx)?;

        if !self.core.manifest.contains_key(&ncx_id) {
            self.core.add_to_manifest(
                ncx_id,
                ncx_href,
                "application/x-dtbncx+xml".to_string(),
                String::new(),
            )?;
        }

        Ok(())
    }

    fn escape_xml(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

fn guess_image_mime(file_name: &str) -> &str {
    match Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => "image/png",
    }
}

fn guess_font_mime(file_name: &str) -> &str {
    match Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "ttf" => "application/font-sfnt",
        "otf" => "application/vnd.ms-opentype",
        "woff" => "application/font-woff",
        "woff2" => "font/woff2",
        _ => "application/font-sfnt",
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
