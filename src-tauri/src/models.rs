use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const WHISPER_DIR: &str = "sherpa-onnx-whisper-tiny.en";
pub const VAD_NAME: &str = "silero_vad.onnx";

const WHISPER_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-whisper-tiny.en.tar.bz2";
const VAD_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub phrases: Vec<String>,
    pub confirm_with_asr: bool,
    pub debounce_seconds: f64,
    pub notify: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            phrases: vec!["i'm sorry".into(), "i am sorry".into()],
            confirm_with_asr: true,
            debounce_seconds: 8.0,
            notify: true,
        }
    }
}

pub fn support_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("PhraseWatch")
}

pub fn models_dir() -> PathBuf {
    if let Ok(p) = std::env::var("PHRASEWATCH_MODELS") {
        return PathBuf::from(p);
    }
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("../Resources/models")))
        .and_then(|p| p.canonicalize().ok());
    if let Some(p) = bundled {
        if p.join(VAD_NAME).exists() {
            return p;
        }
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../models");
    if repo.join(VAD_NAME).exists() {
        return repo;
    }
    support_dir().join("models")
}

pub fn config_path() -> PathBuf {
    support_dir().join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
            return cfg;
        }
    }
    let cfg = AppConfig::default();
    let _ = save_config(&cfg);
    cfg
}

pub fn save_config(cfg: &AppConfig) -> std::io::Result<()> {
    fs::create_dir_all(support_dir())?;
    fs::write(config_path(), serde_json::to_string_pretty(cfg).unwrap() + "\n")
}

pub fn whisper_paths(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let d = root.join(WHISPER_DIR);
    let enc = if d.join("tiny.en-encoder.int8.onnx").exists() {
        d.join("tiny.en-encoder.int8.onnx")
    } else {
        d.join("tiny.en-encoder.onnx")
    };
    let dec = if d.join("tiny.en-decoder.int8.onnx").exists() {
        d.join("tiny.en-decoder.int8.onnx")
    } else {
        d.join("tiny.en-decoder.onnx")
    };
    (enc, dec, d.join("tiny.en-tokens.txt"))
}

pub fn models_ready(root: &Path) -> bool {
    let vad = root.join(VAD_NAME);
    let (wenc, wdec, wtok) = whisper_paths(root);
    vad.exists() && wenc.exists() && wdec.exists() && wtok.exists()
}

fn download(url: &str, dest: &Path, mut on_progress: impl FnMut(u8)) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let resp = ureq::get(url).call().map_err(|e| e.to_string())?;
    let len = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let mut reader = resp.into_reader();
    let tmp = dest.with_extension("part");
    let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 64 * 1024];
    let mut copied = 0u64;
    loop {
        let n = std::io::Read::read(&mut reader, &mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        copied += n as u64;
        if len > 0 {
            on_progress(((copied * 100) / len) as u8);
        }
    }
    fs::rename(tmp, dest).map_err(|e| e.to_string())?;
    Ok(())
}

fn extract_tar_bz2(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(archive).map_err(|e| e.to_string())?;
    let decoder = bzip2::read::BzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn ensure_models(mut on_progress: impl FnMut(u8)) -> Result<PathBuf, String> {
    let root = models_dir();
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    if models_ready(&root) {
        on_progress(100);
        return Ok(root);
    }

    let vad = root.join(VAD_NAME);
    if !vad.exists() {
        on_progress(5);
        download(VAD_URL, &vad, |_| {})?;
    }
    on_progress(25);

    let (wenc, _, _) = whisper_paths(&root);
    if !wenc.exists() {
        let archive = root.join("whisper.tar.bz2");
        download(WHISPER_URL, &archive, |p| on_progress(25 + (p as u16 * 70 / 100) as u8))?;
        extract_tar_bz2(&archive, &root)?;
        let _ = fs::remove_file(archive);
    }
    on_progress(100);
    if !models_ready(&root) {
        return Err("models still missing after download".into());
    }
    Ok(root)
}


