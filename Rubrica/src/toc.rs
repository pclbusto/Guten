use crate::core::GutenCore;
use crate::error::Result;
use crate::types::{DocToc, HeadingItem, TocEntry};
use path_slash::PathExt;
use std::fs;

impl GutenCore {
    /// Escanea un documento XHTML en busca de encabezados (headings) para construir una TOC
    ///
    /// Este método analiza un documento XHTML y extrae todos los encabezados
    /// (`<h1>` a `<h6>`) para generar una tabla de contenidos (Table of Contents).
    /// Es utilizado internamente por [`update_nav`](Self::update_nav) para construir
    /// la navegación automática del EPUB.
    ///
    /// # Proceso de escaneo
    ///
    /// 1. **Resuelve la ruta** - Construye la ruta absoluta al documento usando `opf_dir`
    /// 2. **Lee el archivo** - Carga el contenido XHTML desde el disco
    /// 3. **Parsea el XML** - Usa `roxmltree` para parsear el documento
    /// 4. **Busca encabezados** - Recorre todos los elementos del DOM
    /// 5. **Filtra etiquetas** - Identifica elementos con nombres `<h1>` a `<h6>`
    /// 6. **Extrae información** - Para cada encabezado, extrae:
    ///    - Nivel (1-6)
    ///    - Título (texto interno)
    ///    - Anclaje (atributo `id` para enlaces internos)
    ///
    /// # Argumentos
    ///
    /// * `href` - Ruta relativa al documento XHTML (desde el directorio OPF)
    ///   Ejemplo: `"Text/capitulo1.xhtml"`, `"Text/chapter2.xhtml"`
    ///
    /// # Retorna
    ///
    /// * `Result<DocToc>` - Estructura con la ruta del documento y la lista de encabezados
    ///
    /// # Errores
    ///
    /// * `GutenError::InvalidProject` - Si:
    ///   - `self.opf_dir` es `None` (proyecto no cargado)
    ///   - El archivo no existe o no se puede leer
    ///   - El archivo contiene XML mal formado
    /// * `std::io::Error` - Si falla la lectura del archivo
    pub fn scan_headings(&self, href: &str) -> Result<DocToc> {
        let full_path = self
            .opf_dir
            .as_ref()
            .ok_or_else(|| crate::error::GutenError::InvalidProject("OPF dir not set".to_string()))?
            .join(href);

        let content = fs::read_to_string(full_path)?;

        // 1. Strip DTD (roxmltree doesn't support it)
        let clean_content = self.strip_dtd(&content);

        // 2. Fix void elements (HTML5 -> XHTML) for roxmltree compatibility
        let fixed_content = crate::guardian::html5_to_xhtml_void_elements(&clean_content);

        // 3. Replace common HTML entities that are not predefined in XML
        let normalized_content = Self::normalize_html_entities(&fixed_content);

        let doc = roxmltree::Document::parse(&normalized_content).map_err(|e| {
            crate::error::GutenError::InvalidProject(format!("XML error in {}: {}", href, e))
        })?;

        let mut items = Vec::new();

        for node in doc.descendants().filter(|n| n.is_element()) {
            let tag = node.tag_name().name().to_lowercase();
            if tag.len() == 2 && tag.starts_with('h') {
                if let Ok(level) = tag[1..].parse::<u8>() {
                    if (1..=6).contains(&level) {
                        let title = node
                            .descendants()
                            .filter(|n| n.is_text())
                            .filter_map(|n| n.text())
                            .collect::<Vec<_>>()
                            .join("")
                            .trim()
                            .to_string();
                        let anchor = node.attribute("id").unwrap_or("").to_string();

                        items.push(HeadingItem {
                            level,
                            title,
                            anchor,
                            include: true,
                        });
                    }
                }
            }
        }

        Ok(DocToc {
            href: href.to_string(),
            title: href.to_string(), // Fallback title
            items,
            include: true,
        })
    }

    /// Reemplaza entidades HTML nombradas (que no están predefinidas en XML)
    /// por entidades numéricas equivalentes, para que roxmltree pueda parsear
    /// documentos XHTML que provengan de fuentes HTML o editores poco estrictos.
    fn normalize_html_entities(content: &str) -> String {
        use std::collections::HashMap;
        use std::sync::OnceLock;

        static ENTITIES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
        let entities = ENTITIES.get_or_init(|| {
            let mut m = HashMap::new();
            // XML predefined entities: keep as-is
            m.insert("amp", "&amp;");
            m.insert("lt", "&lt;");
            m.insert("gt", "&gt;");
            m.insert("quot", "&quot;");
            m.insert("apos", "&apos;");
            // Common HTML named entities -> numeric equivalents
            m.insert("nbsp", "&#160;");
            m.insert("copy", "&#169;");
            m.insert("reg", "&#174;");
            m.insert("trade", "&#8482;");
            m.insert("ndash", "&#8211;");
            m.insert("mdash", "&#8212;");
            m.insert("lsquo", "&#8216;");
            m.insert("rsquo", "&#8217;");
            m.insert("ldquo", "&#8220;");
            m.insert("rdquo", "&#8221;");
            m.insert("laquo", "&#171;");
            m.insert("raquo", "&#187;");
            m.insert("hellip", "&#8230;");
            m.insert("bull", "&#8226;");
            m.insert("middot", "&#183;");
            m.insert("eacute", "&#233;");
            m.insert("Eacute", "&#201;");
            m.insert("iacute", "&#237;");
            m.insert("Iacute", "&#205;");
            m.insert("oacute", "&#243;");
            m.insert("Oacute", "&#211;");
            m.insert("uacute", "&#250;");
            m.insert("Uacute", "&#218;");
            m.insert("aacute", "&#225;");
            m.insert("Aacute", "&#193;");
            m.insert("ntilde", "&#241;");
            m.insert("Ntilde", "&#209;");
            m.insert("iquest", "&#191;");
            m.insert("iexcl", "&#161;");
            m.insert("ordf", "&#170;");
            m.insert("ordm", "&#186;");
            m.insert("deg", "&#176;");
            m.insert("plusmn", "&#177;");
            m.insert("para", "&#182;");
            m.insert("sect", "&#167;");
            m.insert("cent", "&#162;");
            m.insert("pound", "&#163;");
            m.insert("yen", "&#165;");
            m.insert("euro", "&#8364;");
            m
        });

        let mut result = String::with_capacity(content.len());
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '&' {
                let mut name = String::new();
                let mut is_entity = true;
                while let Some(&next) = chars.peek() {
                    if next == ';' {
                        chars.next();
                        break;
                    }
                    if next.is_alphanumeric() {
                        name.push(next);
                        chars.next();
                    } else {
                        is_entity = false;
                        break;
                    }
                }

                if is_entity && !name.is_empty() {
                    if let Some(&replacement) = entities.get(name.as_str()) {
                        result.push_str(replacement);
                    } else if name.starts_with('#') {
                        // Already a numeric entity; keep as-is.
                        result.push('&');
                        result.push_str(&name);
                        result.push(';');
                    } else {
                        // Unknown named entity: replace with a space to keep
                        // the parser happy while preserving some readability.
                        result.push(' ');
                    }
                } else {
                    result.push('&');
                    result.push_str(&name);
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Elimina la declaración <!DOCTYPE ...> de un string XML
    /// ya que roxmltree no la soporta y lanza error.
    fn strip_dtd(&self, xml: &str) -> String {
        if let Some(start) = xml.find("<!DOCTYPE") {
            if let Some(end) = xml[start..].find('>') {
                let mut result = xml.to_string();
                result.replace_range(start..start + end + 1, "");
                return result;
            }
        }
        xml.to_string()
    }

    /// Recupera la información de todos los encabezados de todos los capítulos en el spine.
    ///
    /// Este método es ideal para presentárselo al usuario y que este elija qué
    /// elementos desea incluir en la navegación final. Retorna una lista de `DocToc`
    /// preservando el orden del spine.
    ///
    /// # Ejemplo
    /// ```no_run
    /// # use gutencore::GutenCore;
    /// let core = GutenCore::open_folder("./mi_epub")?;
    /// let full_data = core.get_full_toc_data()?;
    ///
    /// for doc in full_data {
    ///     println!("Capítulo: {}", doc.title);
    ///     for h in doc.items {
    ///         println!("  - [{}] {}", h.level, h.title);
    ///     }
    /// }
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_full_toc_data(&self) -> Result<Vec<DocToc>> {
        let mut full_data = Vec::new();
        for idref in &self.spine {
            if let Some(item) = self.manifest.get(idref) {
                if item.media_type == "application/xhtml+xml" {
                    let mut doc_toc = self.scan_headings(&item.href)?;
                    // Usamos el ID del manifiesto como título inicial si no tiene encabezados
                    if doc_toc.items.is_empty() {
                        doc_toc.title = idref.clone();
                    } else {
                        doc_toc.title = doc_toc.items[0].title.clone();
                    }
                    full_data.push(doc_toc);
                }
            }
        }
        Ok(full_data)
    }

    /// Extrae la tabla de contenidos unificada, leyendo tanto `toc.ncx` (EPUB2)
    /// como `nav.xhtml` (EPUB3) y devolviendo una lista plana de entradas.
    ///
    /// Primero intenta parsear `toc.ncx` si existe en el manifiesto.
    /// Si no hay NCX, extrae los enlaces del `<nav epub:type="toc">` en `nav.xhtml`.
    /// Como último recurso, construye la TOC a partir de los encabezados del spine.
    ///
    /// # Retorna
    ///
    /// * `Result<Vec<TocEntry>>` - Lista de entradas con título, href y nivel de profundidad
    pub fn get_toc(&self) -> Result<Vec<TocEntry>> {
        // 1. Try toc.ncx (EPUB2)
        if let Some(entries) = self.parse_ncx_toc()? {
            return Ok(entries);
        }

        // 2. Try nav.xhtml EPUB3 toc
        if let Some(entries) = self.parse_nav_toc()? {
            return Ok(entries);
        }

        // 3. Fallback: build from spine headings
        let full_data = self.get_full_toc_data()?;
        let mut entries = Vec::new();

        let nav_dir = std::path::Path::new("Text");
        for doc in &full_data {
            let doc_path = std::path::Path::new(&doc.href);
            let rel = pathdiff::diff_paths(doc_path, nav_dir)
                .unwrap_or_else(|| doc_path.to_path_buf());
            let rel_str = rel.to_string_lossy();

            for heading in &doc.items {
                let href = if heading.anchor.is_empty() {
                    rel_str.to_string()
                } else {
                    format!("{}#{}", rel_str, heading.anchor)
                };
                entries.push(TocEntry {
                    title: heading.title.clone(),
                    href,
                    level: heading.level,
                });
            }
        }

        Ok(entries)
    }

    /// Intenta parsear `toc.ncx` (EPUB2) y devuelve las entradas de TOC.
    fn parse_ncx_toc(&self) -> Result<Option<Vec<TocEntry>>> {
        let ncx_item = self
            .manifest
            .values()
            .find(|it| it.media_type == "application/x-dtbncx+xml");

        let ncx_item = match ncx_item {
            Some(item) => item,
            None => return Ok(None),
        };

        let opf_dir = self
            .opf_dir
            .as_ref()
            .ok_or_else(|| crate::error::GutenError::InvalidProject("OPF dir not set".to_string()))?;

        let ncx_path = opf_dir.join(&ncx_item.href);
        if !ncx_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&ncx_path)?;
        // Strip DTD — roxmltree doesn't support it
        let clean = self.strip_dtd(&content);
        let doc = roxmltree::Document::parse(&clean).map_err(|e| {
            crate::error::GutenError::InvalidProject(format!("XML error in toc.ncx: {}", e))
        })?;

        let ncx_dir = ncx_path.parent().unwrap();
        let mut entries = Vec::new();

        for nav_point in doc.descendants().filter(|n| n.has_tag_name("navPoint")) {
            // Level = nesting depth (1 + number of navPoint ancestors)
            let level = nav_point
                .ancestors()
                .filter(|n| n.has_tag_name("navPoint"))
                .count() as u8
                + 1;

            let mut title = String::new();
            let mut href = String::new();

            for child in nav_point.children() {
                match child.tag_name().name() {
                    "navLabel" => {
                        if let Some(text_node) = child
                            .children()
                            .find(|n| n.has_tag_name("text"))
                        {
                            title = text_node.text().unwrap_or("").trim().to_string();
                        }
                    }
                    "content" => {
                        let src = child.attribute("src").unwrap_or("");
                        // Resolve relative to the NCX directory
                        let src_path = std::path::Path::new(src);
                        let resolved = ncx_dir.join(src_path);
                        if let Ok(rel) = resolved.strip_prefix(opf_dir) {
                            href = rel.to_string_lossy().replace('\\', "/");
                        } else {
                            href = src.to_string();
                        }
                    }
                    _ => {}
                }
            }

            if !title.is_empty() && !href.is_empty() {
                entries.push(TocEntry {
                    title,
                    href,
                    level,
                });
            }
        }

        if entries.is_empty() {
            Ok(None)
        } else {
            Ok(Some(entries))
        }
    }

    /// Intenta parsear `nav.xhtml` (EPUB3) y extraer las entradas del TOC.
    fn parse_nav_toc(&self) -> Result<Option<Vec<TocEntry>>> {
        let nav_item = self
            .manifest
            .values()
            .find(|it| it.properties.contains("nav"));

        let nav_item = match nav_item {
            Some(item) => item,
            None => return Ok(None),
        };

        let opf_dir = self
            .opf_dir
            .as_ref()
            .ok_or_else(|| crate::error::GutenError::InvalidProject("OPF dir not set".to_string()))?;

        let nav_path = opf_dir.join(&nav_item.href);
        if !nav_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&nav_path)?;

        // Strip DTD
        let clean = if let Some(start) = content.find("<!DOCTYPE") {
            if let Some(end) = content[start..].find('>') {
                let mut s = content.clone();
                s.replace_range(start..start + end + 1, "");
                s
            } else {
                content
            }
        } else {
            content
        };

        let doc = roxmltree::Document::parse(&clean).map_err(|e| {
            crate::error::GutenError::InvalidProject(format!("XML error in nav.xhtml: {}", e))
        })?;

        // Find <nav epub:type="toc">
        let toc_nav = doc
            .descendants()
            .find(|n| {
                n.has_tag_name("nav")
                    && n.attribute("type") == Some("toc")
            });

        let toc_nav = match toc_nav {
            Some(n) => n,
            None => return Ok(None),
        };

        let nav_dir = nav_path.parent().unwrap();
        let mut entries = Vec::new();

        // Recursively collect <a> elements preserving nesting level
        self.collect_nav_links(toc_nav, nav_dir, opf_dir, 1, &mut entries);

        if entries.is_empty() {
            Ok(None)
        } else {
            Ok(Some(entries))
        }
    }

    /// Recolecta recursivamente enlaces `<a>` del árbol de navegación EPUB3.
    fn collect_nav_links(
        &self,
        node: roxmltree::Node,
        nav_dir: &std::path::Path,
        opf_dir: &std::path::Path,
        base_level: u8,
        entries: &mut Vec<TocEntry>,
    ) {
        for child in node.children() {
            if child.has_tag_name("a") {
                if let Some(href) = child.attribute("href") {
                    let title = child.text().unwrap_or("").trim().to_string();
                    if !title.is_empty() {
                        // Resolve relative to the nav directory
                        let href_path = std::path::Path::new(href);
                        let resolved = nav_dir.join(href_path);
                        let href_str = if let Ok(rel) = resolved.strip_prefix(opf_dir) {
                            rel.to_string_lossy().replace('\\', "/")
                        } else {
                            href.to_string()
                        };
                        entries.push(TocEntry {
                            title,
                            href: href_str,
                            level: base_level,
                        });
                    }
                }
            }
            if child.has_tag_name("ol") || child.has_tag_name("ul") {
                self.collect_nav_links(child, nav_dir, opf_dir, base_level + 1, entries);
            } else {
                self.collect_nav_links(child, nav_dir, opf_dir, base_level, entries);
            }
        }
    }

    /// Construye el archivo nav.xhtml basándose en una selección personalizada de datos.
    ///
    /// Este método permite un control total sobre el índice del libro. El usuario puede
    /// filtrar, renombrar o reordenar los elementos antes de llamar a este método.
    ///
    /// # Argumentos
    /// * `data` - Una lista de `DocToc` filtrada y ordenada por el usuario.
    ///
    /// # Errores
    /// * `GutenError::InvalidProject` - Si el directorio OPF no está cargado.
    /// * `std::io::Error` - Si falla la escritura en disco.
    pub fn build_nav_from_data(&mut self, data: &[DocToc]) -> Result<()> {
        let lang = self
            .metadata
            .as_ref()
            .map(|m| m.language.as_str())
            .unwrap_or("es");
        let title = self
            .metadata
            .as_ref()
            .map(|m| m.title.as_str())
            .unwrap_or("Índice");

        let mut html = String::new();
        html.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        html.push_str(&format!(
            "<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" lang=\"{}\" xml:lang=\"{}\">\n",
            lang, lang
        ));
        html.push_str("<head>\n");
        html.push_str("  <meta charset=\"utf-8\"/>\n");
        html.push_str(&format!("  <title>{}</title>\n", title));
        html.push_str("</head>\n");
        html.push_str("<body>\n");
        html.push_str("  <nav epub:type=\"toc\" id=\"toc\">\n");
        html.push_str(&format!("    <h1>{}</h1>\n", title));
        html.push_str("    <ol>\n");

        let nav_dir = std::path::Path::new("Text");

        for doc in data {
            if !doc.include {
                continue;
            }

            let doc_path = std::path::Path::new(&doc.href);
            let rel = pathdiff::diff_paths(doc_path, nav_dir).unwrap_or_else(|| doc_path.to_path_buf());
            let rel_str = rel.to_slash_lossy();

            // Si el documento tiene items internos (h1..h6) y el usuario no los filtró todos
            let has_visible_items = doc.items.iter().any(|h| h.include);

            if has_visible_items {
                for heading in &doc.items {
                    if !heading.include {
                        continue;
                    }

                    let href = if heading.anchor.is_empty() {
                        rel_str.to_string()
                    } else {
                        format!("{}#{}", rel_str, heading.anchor)
                    };

                    html.push_str(&format!(
                        "      <li><a href=\"{}\">{}</a></li>\n",
                        href, heading.title
                    ));
                }
            } else {
                // Si no tiene items internos (o están todos filtrados), pero el doc está incluido
                html.push_str(&format!(
                    "      <li><a href=\"{}\">{}</a></li>\n",
                    rel_str, doc.title
                ));
            }
        }

        html.push_str("    </ol>\n");
        html.push_str("  </nav>\n");
        html.push_str("</body>\n");
        html.push_str("</html>\n");

        // Guardar el archivo
        let opf_dir = self
            .opf_dir
            .as_ref()
            .ok_or_else(|| crate::error::GutenError::InvalidProject("OPF dir not set".into()))?;
        let nav_path = opf_dir.join("Text/nav.xhtml");

        if let Some(parent) = nav_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&nav_path, html)?;

        // Asegurar que esté en el manifiesto con las propiedades correctas
        let nav_id = "nav";
        let nav_href = "Text/nav.xhtml";

        if !self.manifest.contains_key(nav_id) {
            self.add_to_manifest(
                nav_id.to_string(),
                nav_href.to_string(),
                "application/xhtml+xml".to_string(),
                "nav".to_string(),
            )?;
        } else if let Some(item) = self.manifest.get_mut(nav_id) {
            if item.href != nav_href {
                let old_path = opf_dir.join(&item.href);
                let _ = fs::remove_file(old_path);
                item.href = nav_href.to_string();
            }
            item.properties = "nav".to_string();
        }

        Ok(())
    }
}
