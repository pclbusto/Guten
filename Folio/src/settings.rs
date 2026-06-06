use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IlluminationProfile {
    pub name: String,
    pub brightness: f64, // 0.5 - 2.0
    pub contrast: f64,   // 0.5 - 2.0
    pub warmth: f64,     // 0.0 - 1.0 (temperatura de color)
    pub text_color: String,
    pub bg_color: String,
}

impl Default for IlluminationProfile {
    fn default() -> Self {
        Self {
            name: "Día".to_string(),
            brightness: 1.0,
            contrast: 1.0,
            warmth: 0.0,
            text_color: "#1a1a1a".to_string(),
            bg_color: "#faf8f5".to_string(),
        }
    }
}

impl IlluminationProfile {
    pub fn night() -> Self {
        Self {
            name: "Noche".to_string(),
            brightness: 0.8,
            contrast: 1.1,
            warmth: 0.6,
            text_color: "#d8d8d8".to_string(),
            bg_color: "#1a1a1a".to_string(),
        }
    }

    pub fn sepia() -> Self {
        Self {
            name: "Sepia".to_string(),
            brightness: 1.0,
            contrast: 1.0,
            warmth: 0.8,
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
    pub text_align: String, // justify, left
    pub two_page_mode: bool,
    pub current_profile: String,
    pub profiles: HashMap<String, IlluminationProfile>,
    pub dark_mode_smart: bool,
    pub focus_mode: bool,
    pub custom_css: String,
    pub tts_voice: String, // Nombre de la voz seleccionada para TTS
}

impl Default for ReaderSettings {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert("day".to_string(), IlluminationProfile::default());
        profiles.insert("night".to_string(), IlluminationProfile::night());
        profiles.insert("sepia".to_string(), IlluminationProfile::sepia());

        Self {
            font_family: "Georgia, serif".to_string(),
            font_size_pt: 14.0,
            line_height: 1.6,
            margin_em: 2.0,
            text_align: "justify".to_string(),
            two_page_mode: false,
            current_profile: "day".to_string(),
            profiles,
            dark_mode_smart: true,
            focus_mode: false,
            custom_css: String::new(),
            tts_voice: String::new(),
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

    pub fn generate_reader_css(&self) -> String {
        let profile = self.current_profile();
        let two_page = if self.two_page_mode {
            "column-count: 2; column-gap: 3em; column-rule: 1px solid rgba(128,128,128,0.2); column-fill: balance;"
        } else {
            "column-count: 1; column-fill: balance;"
        };

        let warmth_filter = if profile.warmth > 0.0 {
            format!(
                "filter: sepia({}%) brightness({}) contrast({});",
                profile.warmth * 40.0,
                profile.brightness,
                profile.contrast
            )
        } else {
            format!(
                "filter: brightness({}) contrast({});",
                profile.brightness, profile.contrast
            )
        };

        // Modo oscuro inteligente: solo cambia fondo y texto, NO aplica filter a imágenes
        let dark_override = if profile.bg_color == "#1a1a1a" && self.dark_mode_smart {
            format!(
                r#"
                body {{
                    background-color: {} !important;
                    color: {} !important;
                }}
                img, svg, video {{
                    opacity: 0.9;
                    filter: none !important;
                }}
                "#,
                profile.bg_color, profile.text_color
            )
        } else {
            String::new()
        };

        format!(
            r#"
            html {{
                {warmth_filter}
            }}
            body {{
                font-family: {font} !important;
                font-size: {size}pt !important;
                line-height: {lh} !important;
                margin: {margin}em {margin}em {margin}em {margin}em !important;
                text-align: {align} !important;
                background-color: {bg} !important;
                color: {fg} !important;
                {two_page}
                overflow-x: auto;
                min-height: 100vh;
                box-sizing: border-box;
            }}
            img, svg {{
                max-width: 100%;
                height: auto;
                break-inside: avoid;
                page-break-inside: avoid;
            }}
            p {{
                orphans: 2;
                widows: 2;
            }}
            {dark_override}
            {custom}
            "#,
            font = self.font_family,
            size = self.font_size_pt,
            lh = self.line_height,
            margin = self.margin_em,
            align = self.text_align,
            bg = profile.bg_color,
            fg = profile.text_color,
            two_page = two_page,
            warmth_filter = warmth_filter,
            dark_override = dark_override,
            custom = self.custom_css,
        )
    }

    fn settings_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gutenreader")
            .join("settings.json")
    }
}
