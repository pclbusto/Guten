use cosmic::iced::advanced::graphics::text;
use cosmic::iced::font::Family;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::css::FontFaceRule;

#[derive(Debug, Clone, Default)]
pub struct FontNameMap {
    exact: HashMap<(String, bool, bool), Family>,
    fallback: HashMap<String, (Family, bool, bool)>,
}

impl FontNameMap {
    /// Resuelve un nombre CSS a un `Family` cargado, junto con los flags reales
    /// de peso/estilo de la variante que finalmente se encontró.
    pub fn resolve(&self, name: &str, bold: bool, italic: bool) -> Option<(Family, bool, bool)> {
        let norm = normalize_font_name(name);
        eprintln!(
            "[fonts] resolve '{}'(norm={}) bold={} italic={}",
            name, norm, bold, italic
        );

        // 1. coincidencia exacta de peso/estilo
        if self.exact.contains_key(&(norm.clone(), bold, italic)) {
            eprintln!("[fonts]   exact match");
            return Some((self.exact[&(norm.clone(), bold, italic)], bold, italic));
        }

        // 2. si pedimos bold/italic, probar variantes menos específicas
        let variants: Vec<(bool, bool)> = if bold && italic {
            vec![(true, false), (false, true), (false, false)]
        } else if bold {
            vec![(false, false)]
        } else if italic {
            vec![(false, false)]
        } else {
            vec![]
        };
        for (b, i) in variants {
            if self.exact.contains_key(&(norm.clone(), b, i)) {
                eprintln!(
                    "[fonts]   variant fallback match (bold={}, italic={})",
                    b, i
                );
                return Some((self.exact[&(norm.clone(), b, i)], b, i));
            }
        }

        // 3. fallback por nombre normalizado (cualquier variante)
        if let Some((family, b, i)) = self.fallback.get(&norm).copied() {
            eprintln!("[fonts]   name fallback match");
            return Some((family, b, i));
        }

        eprintln!("[fonts]   no match");
        None
    }

    fn insert_variant(&mut self, name: String, bold: bool, italic: bool, family: Family) {
        let norm = normalize_font_name(&name);
        self.exact
            .entry((norm.clone(), bold, italic))
            .or_insert(family);
        // La primera variante que veamos para este nombre sirve como fallback general.
        self.fallback.entry(norm).or_insert((family, bold, italic));
        // También registrar bajo el nombre exacto original, por si no se normaliza.
        self.exact.entry((name, bold, italic)).or_insert(family);
    }
}

#[derive(Debug, Clone)]
pub struct EpubFont {
    pub family_name: String,
    pub family: Family,
    pub bold: bool,
    pub italic: bool,
}

pub fn extract_epub_fonts(
    manifest: &std::collections::HashMap<String, gutencore::ManifestItem>,
    get_resource_path: impl Fn(&str) -> Result<std::path::PathBuf, gutencore::GutenError>,
) -> Vec<EpubFont> {
    let mut fonts = Vec::new();

    for (_id, item) in manifest {
        let mt = item.media_type.to_lowercase();
        if !mt.contains("font")
            && !mt.contains("ttf")
            && !mt.contains("otf")
            && !mt.contains("opentype")
            && !mt.contains("truetype")
            && !mt.contains("woff")
        {
            continue;
        }

        let path = match get_resource_path(&item.href) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let font_info = read_font_info(&bytes);
        let family_name = font_info
            .as_ref()
            .map(|(name, _, _)| name.clone())
            .unwrap_or_else(|| {
                let fallback = Path::new(&item.href)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                fallback
            });

        load_into_font_system(&bytes);
        eprintln!("[fonts] loaded EPUB font '{}' from {:?}", family_name, path);

        let leaked: &'static str = Box::leak(family_name.clone().into_boxed_str());
        fonts.push(EpubFont {
            family_name: family_name.clone(),
            family: Family::Name(leaked),
            bold: font_info.as_ref().is_some_and(|(_, bold, _)| *bold),
            italic: font_info.as_ref().is_some_and(|(_, _, italic)| *italic),
        });
    }

    fonts
}

pub fn build_font_name_map(
    font_faces: &[(FontFaceRule, PathBuf)],
    epub_fonts: &[EpubFont],
) -> FontNameMap {
    let mut map = FontNameMap::default();

    eprintln!(
        "[fonts] building font name map from {} @font-face rules",
        font_faces.len()
    );
    for (face, base_dir) in font_faces {
        let src_path = resolve_css_url(&face.src, base_dir);
        eprintln!(
            "[fonts] @font-face '{}' src resolved to {:?}",
            face.family, src_path
        );
        match fs::read(&src_path) {
            Ok(bytes) => {
                load_into_font_system(&bytes);
                if let Some((real_name, bold, italic)) = read_font_info(&bytes) {
                    let leaked: &'static str = Box::leak(real_name.clone().into_boxed_str());
                    let family = Family::Name(leaked);
                    let is_bold = face.weight >= 700 || bold;
                    let is_italic = face.italic || italic;
                    eprintln!(
                        "[fonts]   registered '{}' -> '{}' (bold={}, italic={})",
                        face.family, real_name, is_bold, is_italic
                    );
                    map.insert_variant(face.family.clone(), is_bold, is_italic, family);
                } else {
                    eprintln!("[fonts]   could not read font info from {:?}", src_path);
                }
            }
            Err(e) => eprintln!("[fonts]   failed to read {:?}: {}", src_path, e),
        }
    }

    for font in epub_fonts {
        // Fallback por si se pide el family name real directamente.
        map.insert_variant(
            font.family_name.clone(),
            font.bold,
            font.italic,
            font.family,
        );
    }

    map
}

fn resolve_css_url(src: &str, base_dir: &Path) -> PathBuf {
    let src = src.trim();
    let src = src.strip_prefix("url(").unwrap_or(src);
    let src = src.strip_suffix(")").unwrap_or(src);
    let src = src.trim().trim_matches(|c: char| c == '\'' || c == '"');
    normalize_path(&base_dir.join(src))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn normalize_font_name(name: &str) -> String {
    name.to_lowercase()
        .replace([' ', '-', '_'], "")
        .replace('"', "")
        .replace('\'', "")
}

fn read_font_info(bytes: &[u8]) -> Option<(String, bool, bool)> {
    let face = ttf_parser::Face::parse(bytes, 0).ok()?;
    // fontdb (and therefore cosmic-text) prefers the OpenType typographic
    // family (name ID 16) over the legacy family (ID 1). Returning ID 1 here
    // can produce a Family::Name that looks valid but never matches the font
    // database, causing a silent fallback to the reader's default font.
    let family = [
        ttf_parser::name_id::TYPOGRAPHIC_FAMILY,
        ttf_parser::name_id::FAMILY,
    ]
    .into_iter()
    .find_map(|name_id| {
        face.names()
            .into_iter()
            .find(|name| name.name_id == name_id && name.is_unicode())
            .and_then(|name| name.to_string())
    })?;
    let bold = face.weight().to_number() >= 700;
    let italic = face.is_italic();
    Some((family, bold, italic))
}

fn load_into_font_system(bytes: &[u8]) {
    match text::font_system().write() {
        Ok(mut fs) => {
            let data: Vec<u8> = bytes.to_vec();
            fs.load_font(Cow::Owned(data));
        }
        Err(e) => eprintln!("[fonts] failed to acquire font system lock: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::FontFaceRule;
    use std::path::PathBuf;

    #[test]
    #[ignore]
    fn build_nikona_font_map() {
        // Busca la carpeta de caché que contiene el EPUB con Nikona.
        let cache = PathBuf::from("/home/pedro/.cache/gutenreader/epubs");
        let mut target: Option<PathBuf> = None;
        if cache.exists() {
            for entry in std::fs::read_dir(&cache).unwrap() {
                let entry = entry.unwrap();
                let fonts_dir = entry.path().join("OEBPS/Fonts");
                if fonts_dir.join("Nikona.otf").exists() && fonts_dir.join("Nikona-B.otf").exists()
                {
                    target = Some(entry.path().join("OEBPS/Styles"));
                    break;
                }
            }
        }
        let styles_dir = target.expect("no cached EPUB with Nikona fonts");

        let faces = vec![
            (
                FontFaceRule {
                    family: "Nikona".into(),
                    src: "../Fonts/Nikona.otf".into(),
                    weight: 400,
                    italic: false,
                },
                styles_dir.clone(),
            ),
            (
                FontFaceRule {
                    family: "Nikona-B".into(),
                    src: "../Fonts/Nikona-B.otf".into(),
                    weight: 700,
                    italic: false,
                },
                styles_dir,
            ),
        ];

        let map = build_font_name_map(&faces, &[]);
        let nikona = map.resolve("Nikona", false, false);
        let nikona_bold = map.resolve("Nikona", true, false);
        let nikona_b = map.resolve("Nikona-B", true, false);
        assert!(nikona.is_some(), "Nikona should resolve");
        assert_eq!(
            nikona.unwrap().1,
            false,
            "Nikona should resolve to non-bold variant"
        );
        assert!(
            nikona_bold.is_some(),
            "Nikona bold should fall back to Nikona regular"
        );
        assert_eq!(
            nikona_bold.unwrap().1,
            false,
            "Nikona should keep the actual regular font weight"
        );
        assert!(nikona_b.is_some(), "Nikona-B should resolve");
        assert_eq!(
            nikona_b.unwrap().1,
            true,
            "Nikona-B should resolve to bold variant"
        );
    }

    #[test]
    fn separate_css_families_do_not_alias_by_suffix() {
        let mut map = FontNameMap::default();
        let nikona = Family::Name("Nikona");
        let nikona_b = Family::Name("Nikona-B");

        map.insert_variant("Nikona".into(), false, false, nikona);
        map.insert_variant("Nikona-B".into(), true, false, nikona_b);

        assert_eq!(
            map.resolve("Nikona", false, false),
            Some((nikona, false, false))
        );
        assert_eq!(
            map.resolve("Nikona", true, false),
            Some((nikona, false, false))
        );
        assert_eq!(
            map.resolve("Nikona-B", false, false),
            Some((nikona_b, true, false))
        );
        assert_eq!(
            map.resolve("Nikona-B", true, false),
            Some((nikona_b, true, false))
        );
    }

    #[test]
    fn same_css_family_uses_declared_weight_variants() {
        let mut map = FontNameMap::default();
        let din_regular = Family::Name("DINNextLTPro");
        let din_bold = Family::Name("DINNextLTPro Bold");

        map.insert_variant("DINNextLTPro".into(), false, false, din_regular);
        map.insert_variant("DINNextLTPro".into(), true, false, din_bold);

        assert_eq!(
            map.resolve("DINNextLTPro", false, false),
            Some((din_regular, false, false))
        );
        assert_eq!(
            map.resolve("DINNextLTPro", true, false),
            Some((din_bold, true, false))
        );
    }
}
