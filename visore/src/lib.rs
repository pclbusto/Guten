pub mod viewer;
pub mod crop;
pub mod sidebar;

use cosmic::iced::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AspectRatio {
    Free,
    Original,
    Square,
    Ratio5x4,
    Ratio4x3,
    Ratio3x2,
    Ratio16x9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Landscape,
    Portrait,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerTheme {
    Light,
    Dark,
}

impl ViewerTheme {
    pub fn key(&self) -> &str {
        match self {
            ViewerTheme::Light => "light",
            ViewerTheme::Dark => "dark",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key {
            "light" => ViewerTheme::Light,
            _ => ViewerTheme::Dark,
        }
    }

    pub fn bg_color(&self) -> Color {
        match self {
            ViewerTheme::Light => Color::from_rgb8(245, 245, 248),
            ViewerTheme::Dark => Color::from_rgb8(18, 18, 22),
        }
    }

    pub fn sidebar_bg(&self) -> Color {
        match self {
            ViewerTheme::Light => Color::from_rgb8(235, 235, 240),
            ViewerTheme::Dark => Color::from_rgb8(38, 38, 44),
        }
    }

    pub fn text_color(&self) -> Color {
        match self {
            ViewerTheme::Light => Color::from_rgb8(30, 30, 35),
            ViewerTheme::Dark => Color::from_rgb8(220, 220, 230),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ViewerState {
    pub image_path: Option<std::path::PathBuf>,
    pub image_handle: Option<cosmic::widget::image::Handle>,
    pub image_original: Option<image::DynamicImage>,
    pub title: String,

    pub theme: ViewerTheme,

    pub rotation: i32,
    pub flip_h: bool,
    pub flip_v: bool,

    pub crop_enabled: bool,
    pub crop_rect: CropRect,
    pub aspect_ratio: AspectRatio,
    pub orientation: Orientation,
}

#[derive(Debug, Clone, Copy)]
pub struct CropRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Default for CropRect {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }
    }
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            image_path: None,
            image_handle: None,
            image_original: None,
            title: String::new(),
            theme: ViewerTheme::Dark,
            rotation: 0,
            flip_h: false,
            flip_v: false,
            crop_enabled: false,
            crop_rect: CropRect::default(),
            aspect_ratio: AspectRatio::Free,
            orientation: Orientation::Landscape,
        }
    }
}

impl ViewerState {
    pub fn load_image(&mut self, path: &std::path::Path) -> Result<(), String> {
        let img = image::open(path).map_err(|e| format!("Error cargando imagen: {}", e))?;
        self.image_original = Some(img.clone());
        self.title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Imagen")
            .to_string();
        self.image_path = Some(path.to_path_buf());
        self.rotation = 0;
        self.flip_h = false;
        self.flip_v = false;
        self.crop_rect = CropRect::default();
        self.update_handle();
        Ok(())
    }

    pub fn update_handle(&mut self) {
        if let Some(ref img) = self.image_original {
            let mut processed = img.clone();

            if self.rotation != 0 {
                processed = match self.rotation {
                    90 | -270 => processed.rotate90(),
                    180 | -180 => processed.rotate180(),
                    270 | -90 => processed.rotate270(),
                    _ => processed,
                };
            }
            if self.flip_h {
                processed = processed.fliph();
            }
            if self.flip_v {
                processed = processed.flipv();
            }

            let rgba = processed.to_rgba8();
            let (w, h) = rgba.dimensions();
            let pixels = rgba.into_raw();

            self.image_handle = Some(cosmic::widget::image::Handle::from_rgba(
                w, h, pixels,
            ));
        }
    }

    pub fn rotate_cw(&mut self) {
        self.rotation = (self.rotation + 90) % 360;
        self.update_handle();
    }

    pub fn rotate_ccw(&mut self) {
        self.rotation = (self.rotation - 90) % 360;
        self.update_handle();
    }

    pub fn toggle_flip_h(&mut self) {
        self.flip_h = !self.flip_h;
        self.update_handle();
    }

    pub fn toggle_flip_v(&mut self) {
        self.flip_v = !self.flip_v;
        self.update_handle();
    }
}

pub fn dark_bg() -> Color {
    Color::from_rgb8(28, 28, 32)
}

pub fn accent_color() -> Color {
    Color::from_rgb8(220, 80, 140)
}
