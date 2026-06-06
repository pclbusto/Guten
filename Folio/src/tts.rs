use std::cell::RefCell;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

const MAX_TTS_CHARS: usize = 1400;

/// Información sobre una voz TTS disponible
#[derive(Debug, Clone)]
pub struct VoiceInfo {
    pub name: String,
    pub language: String,
    pub config_path: PathBuf,
    pub onnx_path: PathBuf,
}

/// Motor TTS que usa el mejor backend disponible:
/// 1. piper (neural, excelente calidad) — requiere instalar piper + modelo
/// 2. espeak-ng (mejor que espeak clásico)
/// 3. spd-say / espeak (fallback)
pub struct TtsEngine {
    speaking: AtomicBool,
    current_child: RefCell<Option<Child>>,
    backend: RefCell<TtsBackend>,
    selected_voice: RefCell<Option<VoiceInfo>>,
    voices: RefCell<Vec<VoiceInfo>>,
    speed: RefCell<f64>,
    current_text: RefCell<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TtsBackend {
    Piper,
    EspeakNg,
    SpdSay,
    Espeak,
    None,
}

impl TtsEngine {
    pub fn new() -> Self {
        let voices = Self::scan_piper_voices();
        let backend = if find_piper_binary().is_some() && !voices.is_empty() {
            TtsBackend::Piper
        } else if which_command("espeak-ng") {
            TtsBackend::EspeakNg
        } else if which_command("spd-say") {
            TtsBackend::SpdSay
        } else if which_command("espeak") {
            TtsBackend::Espeak
        } else {
            TtsBackend::None
        };

        let selected_voice = voices.first().cloned();

        eprintln!("[TTS] Backend: {:?}, Voces: {}", backend, voices.len());
        for v in &voices {
            eprintln!(
                "[TTS]   - {} ({}) [{:?}]",
                v.name, v.language, v.config_path
            );
        }

        Self {
            speaking: AtomicBool::new(false),
            current_child: RefCell::new(None),
            backend: RefCell::new(backend),
            selected_voice: RefCell::new(selected_voice),
            voices: RefCell::new(voices),
            speed: RefCell::new(1.0),
            current_text: RefCell::new(String::new()),
        }
    }

    /// Busca modelos de Piper en múltiples rutas posibles
    fn scan_piper_voices() -> Vec<VoiceInfo> {
        let mut voices = Vec::new();
        let mut search_dirs = Vec::new();

        // Ruta estándar
        if let Some(data_dir) = dirs::data_dir() {
            search_dirs.push(data_dir.join("piper-tts"));
        }
        // Ruta alternativa en home
        if let Some(home) = dirs::home_dir() {
            search_dirs.push(home.join("piper-tts"));
            search_dirs.push(home.join(".piper-tts"));
        }
        // Ruta del AUR (piper-tts-bin instala en /opt/piper-tts)
        search_dirs.push(PathBuf::from("/opt/piper-tts"));

        for dir in search_dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.extension() == Some(std::ffi::OsStr::new("json")) {
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");
                        // Remover .onnx del nombre si está presente
                        let name = name.trim_end_matches(".onnx").to_string();
                        let onnx_path = path.with_extension(""); // remueve .json
                        let onnx_path =
                            if onnx_path.extension() == Some(std::ffi::OsStr::new("onnx")) {
                                onnx_path
                            } else {
                                path.with_extension("onnx")
                            };

                        if onnx_path.exists() {
                            // Intentar leer el JSON para obtener idioma
                            let lang = std::fs::read_to_string(&path)
                                .ok()
                                .and_then(|content| {
                                    serde_json::from_str::<serde_json::Value>(&content).ok()
                                })
                                .and_then(|json| {
                                    json.get("language")
                                        .and_then(|l| l.get("code"))
                                        .or_else(|| json.get("espeak"))
                                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                                })
                                .unwrap_or_else(|| "unknown".to_string());

                            voices.push(VoiceInfo {
                                name,
                                language: lang,
                                config_path: path,
                                onnx_path,
                            });
                        }
                    }
                }
            }
        }

        voices.sort_by(|a, b| a.name.cmp(&b.name));
        voices
    }

    pub fn voices(&self) -> Vec<VoiceInfo> {
        self.voices.borrow().clone()
    }

    pub fn selected_voice(&self) -> Option<VoiceInfo> {
        self.selected_voice.borrow().clone()
    }

    pub fn set_voice(&self, voice_name: &str) {
        let voices = self.voices.borrow();
        if let Some(voice) = voices.iter().find(|v| v.name == voice_name).cloned() {
            *self.selected_voice.borrow_mut() = Some(voice);
            eprintln!("[TTS] Voz seleccionada: {}", voice_name);
        }
    }

    pub fn speed(&self) -> f64 {
        *self.speed.borrow()
    }

    pub fn increase_speed(&self) -> f64 {
        let mut s = self.speed.borrow_mut();
        *s = (*s + 0.1).min(2.0);
        eprintln!("[TTS] Velocidad aumentada: {:.1}x", *s);
        *s
    }

    pub fn decrease_speed(&self) -> f64 {
        let mut s = self.speed.borrow_mut();
        *s = (*s - 0.1).max(0.5);
        eprintln!("[TTS] Velocidad disminuida: {:.1}x", *s);
        *s
    }

    pub fn restart(&self) -> anyhow::Result<()> {
        let text = self.current_text.borrow().clone();
        if self.is_speaking() && !text.is_empty() {
            let _ = self.stop();
            self.speak(&text)?;
        }
        Ok(())
    }

    pub fn speak(&self, text: &str) -> anyhow::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let text = truncate_text(text, MAX_TTS_CHARS);
        *self.current_text.borrow_mut() = text.to_string();

        let _ = self.stop();

        let backend = *self.backend.borrow();
        let speed = *self.speed.borrow();
        let child = match backend {
            TtsBackend::Piper => {
                if let Some(child) = self.spawn_piper(text, speed) {
                    child
                } else {
                    eprintln!("[TTS] Piper falló, usando espeak-ng...");
                    let rate = (175.0 * speed) as i32;
                    let mut command = Command::new("espeak-ng");
                    prepare_child_command(
                        command
                            .arg("-v")
                            .arg("es")
                            .arg("-s")
                            .arg(rate.to_string())
                            .arg(text)
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null()),
                    )
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("TTS error: {}", e))?
                }
            }
            TtsBackend::EspeakNg => {
                let rate = (175.0 * speed) as i32;
                let mut command = Command::new("espeak-ng");
                prepare_child_command(
                    command
                        .arg("-v")
                        .arg("es")
                        .arg("-s")
                        .arg(rate.to_string())
                        .arg(text)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null()),
                )
                .spawn()
                .map_err(|e| anyhow::anyhow!("TTS error: {}", e))?
            }
            TtsBackend::SpdSay => {
                let rate = ((speed - 1.0) * 100.0) as i32;
                let mut command = Command::new("spd-say");
                prepare_child_command(
                    command
                        .arg("-w")
                        .arg("-r")
                        .arg(rate.to_string())
                        .arg(text)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null()),
                )
                .spawn()
                .map_err(|e| anyhow::anyhow!("TTS error: {}", e))?
            }
            TtsBackend::Espeak => {
                let rate = (170.0 * speed) as i32;
                let mut command = Command::new("espeak");
                prepare_child_command(
                    command
                        .arg("-s")
                        .arg(rate.to_string())
                        .arg(text)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null()),
                )
                .spawn()
                .map_err(|e| anyhow::anyhow!("TTS error: {}", e))?
            }
            TtsBackend::None => return Ok(()),
        };

        self.speaking.store(true, Ordering::SeqCst);
        *self.current_child.borrow_mut() = Some(child);
        Ok(())
    }

    fn spawn_piper(&self, text: &str, speed: f64) -> Option<Child> {
        let voice = self.selected_voice.borrow().clone()?;
        let piper_bin = find_piper_binary()?;
        let length_scale = (1.0 / speed).clamp(0.5, 2.0);

        let script = r#"
tmp="${TMPDIR:-/tmp}/gutenreader_tts_$$.wav"
trap 'rm -f "$tmp"' EXIT INT TERM
"$1" --model "$2" --config "$3" --length_scale "$4" --output_file "$tmp"
status=$?
if [ "$status" -ne 0 ]; then
    "$1" --model "$2" --config "$3" --output_file "$tmp"
    status=$?
fi
if [ "$status" -eq 0 ]; then
    if command -v aplay >/dev/null 2>&1; then
        aplay "$tmp"
    elif command -v paplay >/dev/null 2>&1; then
        paplay "$tmp"
    fi
fi
"#;

        let mut command = Command::new("sh");
        let mut child = prepare_child_command(
            command
                .arg("-c")
                .arg(script)
                .arg("gutenreader-piper")
                .arg(piper_bin)
                .arg(&voice.onnx_path)
                .arg(&voice.config_path)
                .arg(format!("{length_scale:.2}"))
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
        )
        .spawn()
        .ok()?;

        if let Some(stdin) = child.stdin.take() {
            let text = text.to_string();
            std::thread::spawn(move || {
                use std::io::Write;
                let mut stdin = stdin;
                let _ = stdin.write_all(text.as_bytes());
                let _ = stdin.flush();
            });
        }

        Some(child)
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.speaking.store(false, Ordering::SeqCst);

        if let Some(mut child) = self.current_child.borrow_mut().take() {
            terminate_child(&mut child);
        }

        if which_command("spd-say") {
            let _ = Command::new("spd-say")
                .arg("-S")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        Ok(())
    }

    pub fn is_speaking(&self) -> bool {
        if self.speaking.load(Ordering::SeqCst) {
            let mut child_ref = self.current_child.borrow_mut();
            let finished = match child_ref.as_mut().map(|child| child.try_wait()) {
                Some(Ok(Some(_))) => true,
                Some(Err(_)) => true,
                _ => false,
            };
            if finished {
                child_ref.take();
                self.speaking.store(false, Ordering::SeqCst);
            }
        }

        self.speaking.load(Ordering::SeqCst)
    }
}

fn prepare_child_command(command: &mut Command) -> &mut Command {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
}

fn terminate_child(child: &mut Child) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .arg("-TERM")
            .arg(format!("-{}", child.id()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = Command::new("kill")
            .arg("-KILL")
            .arg(format!("-{}", child.id()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    let _ = child.kill();
    let _ = child.try_wait();
}

fn truncate_text(text: &str, max_chars: usize) -> &str {
    if text.chars().count() <= max_chars {
        return text;
    }

    let end = text
        .char_indices()
        .nth(max_chars)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len());
    &text[..end]
}

fn which_command(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Devuelve la ruta al binario piper si existe
fn find_piper_binary() -> Option<PathBuf> {
    if which_command("piper") {
        return Some(PathBuf::from("piper"));
    }
    // El AUR piper-tts-bin instala en /opt/piper-tts/
    let alt = PathBuf::from("/opt/piper-tts/piper");
    if alt.exists() {
        return Some(alt);
    }
    None
}
