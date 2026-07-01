use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Default)]
pub struct ImageMetadataCache {
    entries: HashMap<String, Option<(u32, u32)>>,
}

impl ImageMetadataCache {
    pub fn dimensions(&mut self, path: &str) -> Option<(u32, u32)> {
        if let Some(dimensions) = self.entries.get(path) {
            return *dimensions;
        }

        let dimensions = image_dimensions(path);
        self.entries.insert(path.to_owned(), dimensions);
        dimensions
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Lee únicamente la cabecera de formatos raster soportados por el lector.
/// SVG queda deliberadamente fuera de v0.3 y retorna `None`.
pub fn image_dimensions(path: &str) -> Option<(u32, u32)> {
    let bytes = fs::read(path).ok()?;
    raster_dimensions(&bytes)
}

pub fn scaled_image_size(
    image_width: u32,
    image_height: u32,
    content_width: f32,
    max_height: f32,
) -> Option<(f32, f32)> {
    if image_width == 0 || image_height == 0 || content_width <= 0.0 || max_height <= 0.0 {
        return None;
    }

    let width = image_width as f32;
    let height = image_height as f32;
    let scale = 1.0_f32.min(content_width / width).min(max_height / height);
    Some((width * scale, height * scale))
}

pub fn placeholder_label(path: &str, alt: &str) -> String {
    let description = (!alt.is_empty()).then_some(alt);
    let is_svg = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"));

    match (is_svg, description) {
        (true, Some(description)) => format!("SVG no soportado: {description}"),
        (true, None) => "SVG no soportado".to_owned(),
        (false, Some(description)) => format!("Imagen no disponible: {description}"),
        (false, None) => "Imagen no disponible".to_owned(),
    }
}

fn raster_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() >= 24 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some((
            u32::from_be_bytes(bytes[16..20].try_into().ok()?),
            u32::from_be_bytes(bytes[20..24].try_into().ok()?),
        ));
    }

    if bytes.len() >= 10 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
        return Some((
            u16::from_le_bytes(bytes[6..8].try_into().ok()?) as u32,
            u16::from_le_bytes(bytes[8..10].try_into().ok()?) as u32,
        ));
    }

    if bytes.starts_with(&[0xff, 0xd8]) {
        return jpeg_dimensions(bytes);
    }

    webp_dimensions(bytes)
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut pos = 2;
    while pos + 3 < bytes.len() {
        if bytes[pos] != 0xff {
            pos += 1;
            continue;
        }
        while pos < bytes.len() && bytes[pos] == 0xff {
            pos += 1;
        }
        let marker = *bytes.get(pos)?;
        pos += 1;
        if matches!(marker, 0x01 | 0xd8 | 0xd9) {
            continue;
        }

        let segment_len = u16::from_be_bytes(bytes.get(pos..pos + 2)?.try_into().ok()?) as usize;
        if segment_len < 2 || pos + segment_len > bytes.len() {
            return None;
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            let height = u16::from_be_bytes(bytes.get(pos + 3..pos + 5)?.try_into().ok()?);
            let width = u16::from_be_bytes(bytes.get(pos + 5..pos + 7)?.try_into().ok()?);
            return Some((width as u32, height as u32));
        }
        pos += segment_len;
    }
    None
}

fn webp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 30 || !bytes.starts_with(b"RIFF") || &bytes[8..12] != b"WEBP" {
        return None;
    }

    match &bytes[12..16] {
        b"VP8X" => {
            let width = 1 + u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]);
            let height = 1 + u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]);
            Some((width, height))
        }
        b"VP8L" if bytes.get(20) == Some(&0x2f) && bytes.len() >= 25 => {
            let width = 1 + u32::from(bytes[21]) + (u32::from(bytes[22] & 0x3f) << 8);
            let height = 1
                + (u32::from(bytes[22] >> 6)
                    | (u32::from(bytes[23]) << 2)
                    | (u32::from(bytes[24] & 0x0f) << 10));
            Some((width, height))
        }
        b"VP8 " => bytes[20..]
            .windows(7)
            .find(|window| window.starts_with(&[0x9d, 0x01, 0x2a]))
            .map(|window| {
                let width = u16::from_le_bytes([window[3], window[4]]) & 0x3fff;
                let height = u16::from_le_bytes([window[5], window[6]]) & 0x3fff;
                (width as u32, height as u32)
            }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageMetadataCache, placeholder_label, raster_dimensions, scaled_image_size};

    #[test]
    fn reads_png_dimensions() {
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[16..20].copy_from_slice(&640_u32.to_be_bytes());
        png[20..24].copy_from_slice(&480_u32.to_be_bytes());
        assert_eq!(raster_dimensions(&png), Some((640, 480)));
    }

    #[test]
    fn reads_jpeg_dimensions() {
        let mut jpeg = vec![0; 21];
        jpeg[..6].copy_from_slice(&[0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11]);
        jpeg[6] = 8;
        jpeg[7..9].copy_from_slice(&720_u16.to_be_bytes());
        jpeg[9..11].copy_from_slice(&1280_u16.to_be_bytes());
        assert_eq!(raster_dimensions(&jpeg), Some((1280, 720)));
    }

    #[test]
    fn reads_webp_dimensions() {
        let mut webp = vec![0; 30];
        webp[..4].copy_from_slice(b"RIFF");
        webp[8..12].copy_from_slice(b"WEBP");
        webp[12..16].copy_from_slice(b"VP8X");
        webp[24..27].copy_from_slice(&[0xff, 0x03, 0x00]);
        webp[27..30].copy_from_slice(&[0xff, 0x01, 0x00]);
        assert_eq!(raster_dimensions(&webp), Some((1024, 512)));
    }

    #[test]
    fn scales_without_upscaling_and_respects_limits() {
        assert_eq!(
            scaled_image_size(1200, 600, 600.0, 500.0),
            Some((600.0, 300.0))
        );
        assert_eq!(
            scaled_image_size(600, 1200, 800.0, 600.0),
            Some((300.0, 600.0))
        );
        assert_eq!(
            scaled_image_size(200, 100, 600.0, 500.0),
            Some((200.0, 100.0))
        );
    }

    #[test]
    fn svg_placeholder_is_explicit() {
        assert_eq!(
            placeholder_label("cover.svg", "Portada"),
            "SVG no soportado: Portada"
        );
    }

    #[test]
    fn metadata_cache_does_not_read_the_same_path_twice() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "folio-image-metadata-{}-{unique}.png",
            std::process::id()
        ));
        let mut png = vec![0; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[16..20].copy_from_slice(&320_u32.to_be_bytes());
        png[20..24].copy_from_slice(&240_u32.to_be_bytes());
        std::fs::write(&path, png).unwrap();

        let mut cache = ImageMetadataCache::default();
        let path = path.to_string_lossy().into_owned();
        assert_eq!(cache.dimensions(&path), Some((320, 240)));

        std::fs::write(&path, []).unwrap();
        assert_eq!(cache.dimensions(&path), Some((320, 240)));
        std::fs::remove_file(path).unwrap();
    }
}
