use crate::matcher::{match_phrase, Debouncer};
use crate::models::{models_dir, whisper_paths, AppConfig, VAD_NAME};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use serde::Serialize;
use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig, OfflineWhisperModelConfig, SileroVadModelConfig,
    VadModelConfig, VoiceActivityDetector, Wave,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize)]
pub struct Hit {
    pub phrase: String,
    pub source: String,
    pub transcript: String,
    pub t: f64,
}

pub struct Engine {
    listening: Arc<AtomicBool>,
    hits: Arc<Mutex<Vec<Hit>>>,
    stop_tx: Mutex<Option<Sender<()>>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            listening: Arc::new(AtomicBool::new(false)),
            hits: Arc::new(Mutex::new(Vec::new())),
            stop_tx: Mutex::new(None),
        }
    }

    pub fn is_listening(&self) -> bool {
        self.listening.load(Ordering::SeqCst)
    }

    pub fn hits(&self) -> Vec<Hit> {
        self.hits.lock().unwrap().clone()
    }

    pub fn stop(&self) {
        self.listening.store(false, Ordering::SeqCst);
        if let Some(tx) = self.stop_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    pub fn start<F>(&self, cfg: AppConfig, on_hit: F) -> Result<(), String>
    where
        F: Fn(Hit) + Send + 'static,
    {
        self.stop();
        self.listening.store(true, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        *self.stop_tx.lock().unwrap() = Some(tx);
        let listening = self.listening.clone();
        let hits = self.hits.clone();
        thread::spawn(move || {
            if let Err(err) = run_loop(cfg, listening.clone(), hits, on_hit, rx) {
                eprintln!("PhraseWatch engine error: {err}");
            }
            listening.store(false, Ordering::SeqCst);
        });
        Ok(())
    }
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn resample(samples: &[f32], src: u32, dst: u32) -> Vec<f32> {
    if src == dst || samples.is_empty() {
        return samples.to_vec();
    }
    let n_dst = ((samples.len() as f64) * f64::from(dst) / f64::from(src)).round() as usize;
    if n_dst == 0 {
        return Vec::new();
    }
    let last = samples.len().saturating_sub(1).max(1);
    (0..n_dst)
        .map(|i| {
            let x = i as f64 * last as f64 / (n_dst.saturating_sub(1).max(1) as f64);
            let i0 = x.floor() as usize;
            let i1 = (i0 + 1).min(samples.len() - 1);
            let t = (x - i0 as f64) as f32;
            samples[i0] * (1.0 - t) + samples[i1] * t
        })
        .collect()
}

fn build_asr(root: &Path) -> Result<OfflineRecognizer, String> {
    let (enc, dec, tokens) = whisper_paths(root);
    let mut config = OfflineRecognizerConfig::default();
    config.model_config.whisper = OfflineWhisperModelConfig {
        encoder: Some(enc.to_string_lossy().into()),
        decoder: Some(dec.to_string_lossy().into()),
        language: Some("en".into()),
        task: Some("transcribe".into()),
        tail_paddings: 200,
        enable_token_timestamps: false,
        enable_segment_timestamps: false,
    };
    config.model_config.tokens = Some(tokens.to_string_lossy().into());
    config.model_config.provider = Some("cpu".into());
    config.model_config.num_threads = 2;
    OfflineRecognizer::create(&config).ok_or_else(|| "failed to create whisper recognizer".into())
}

fn transcribe(asr: &OfflineRecognizer, samples: &[f32], sample_rate: i32) -> String {
    let stream = asr.create_stream();
    stream.accept_waveform(sample_rate, samples);
    asr.decode(&stream);
    stream.get_result().map(|r| r.text).unwrap_or_default()
}

pub fn process_wav(path: &Path, cfg: &AppConfig) -> Result<Vec<Hit>, String> {
    let root = models_dir();
    let asr = build_asr(&root)?;
    let wave = Wave::read(path.to_string_lossy().as_ref()).ok_or("failed to read wav")?;
    let transcript = transcribe(&asr, wave.samples(), wave.sample_rate());
    let mut hits = Vec::new();
    if let Some(matched) = match_phrase(&transcript, &cfg.phrases) {
        hits.push(Hit {
            phrase: matched.to_string(),
            source: "asr".into(),
            transcript,
            t: now_secs(),
        });
    }
    Ok(hits)
}

fn run_loop<F>(
    cfg: AppConfig,
    listening: Arc<AtomicBool>,
    hits_store: Arc<Mutex<Vec<Hit>>>,
    on_hit: F,
    stop_rx: mpsc::Receiver<()>,
) -> Result<(), String>
where
    F: Fn(Hit),
{
    let root = models_dir();
    let asr = build_asr(&root)?;
    let mut silero = SileroVadModelConfig::default();
    silero.model = Some(root.join(VAD_NAME).to_string_lossy().into());
    silero.threshold = 0.5;
    silero.min_silence_duration = 0.25;
    silero.min_speech_duration = 0.15;
    let vad_config = VadModelConfig {
        silero_vad: silero,
        ten_vad: Default::default(),
        sample_rate: 16000,
        num_threads: 1,
        provider: Some("cpu".into()),
        debug: false,
    };
    let vad = VoiceActivityDetector::create(&vad_config, 30.0)
        .ok_or_else(|| "failed to create VAD".to_string())?;

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no microphone".to_string())?;
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let sample_rate = supported.sample_rate().0;
    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let stream = build_input_stream(&device, tx)?;
    stream.play().map_err(|e| e.to_string())?;

    let mut debouncer = Debouncer::new(cfg.debounce_seconds);
    let mut leftover: Vec<f32> = Vec::new();

    loop {
        if stop_rx.try_recv().is_ok() || !listening.load(Ordering::SeqCst) {
            break;
        }
        let Ok(chunk) = rx.recv_timeout(std::time::Duration::from_millis(200)) else {
            continue;
        };
        leftover.extend(resample(&chunk, sample_rate, 16000));
        while leftover.len() >= 512 {
            let window: Vec<f32> = leftover.drain(..512).collect();
            vad.accept_waveform(&window);
            while let Some(seg) = vad.front() {
                let samples = seg.samples().to_vec();
                vad.pop();
                let transcript = transcribe(&asr, &samples, 16000);
                if let Some(matched) = match_phrase(&transcript, &cfg.phrases) {
                    if !debouncer.allow(matched) {
                        continue;
                    }
                    let hit = Hit {
                        phrase: matched.to_string(),
                        source: "vad+asr".into(),
                        transcript,
                        t: now_secs(),
                    };
                    hits_store.lock().unwrap().push(hit.clone());
                    on_hit(hit);
                }
            }
        }
    }
    Ok(())
}

fn build_input_stream(device: &cpal::Device, tx: Sender<Vec<f32>>) -> Result<cpal::Stream, String> {
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let config = supported.config();
    let sample_format = supported.sample_format();
    let channels = config.channels as usize;
    let err_fn = |err| eprintln!("audio error: {err}");
    let stream = match sample_format {
        SampleFormat::F32 => device
            .build_input_stream(
                &config,
                move |data: &[f32], _| {
                    let _ = tx.send(mono_f32(data, channels));
                },
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        SampleFormat::I16 => device
            .build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let samples: Vec<f32> = data.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
                    let _ = tx.send(mono_f32(&samples, channels));
                },
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        other => return Err(format!("unsupported sample format {other:?}")),
    };
    Ok(stream)
}

fn mono_f32(data: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }
    data.chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

pub fn notify_macos(hit: &Hit) {
    let body = format!("You said “{}”", hit.phrase);
    let script = format!(
        r#"display notification "{}" with title "PhraseWatch" sound name "Ping""#,
        body.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status();
}

#[allow(dead_code)]
pub fn process_wav_path(path: PathBuf, cfg: AppConfig) -> Result<Vec<Hit>, String> {
    process_wav(&path, &cfg)
}
