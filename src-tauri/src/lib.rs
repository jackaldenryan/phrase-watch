pub mod engine;
pub mod matcher;
pub mod models;
pub mod stats;

use engine::{notify_macos, Engine, Hit};
use models::{
    ensure_models as download_models, load_config, models_dir, models_ready, save_config, AppConfig,
};
use stats::StatsResult;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};

struct AppState {
    engine: Mutex<Engine>,
    config: Mutex<AppConfig>,
}

#[derive(Serialize)]
struct UiState {
    listening: bool,
    models_ready: bool,
    phrases: Vec<String>,
    hits: Vec<Hit>,
    version: String,
}

#[tauri::command]
fn get_state(state: State<AppState>) -> UiState {
    let engine = state.engine.lock().unwrap();
    let cfg = state.config.lock().unwrap();
    UiState {
        listening: engine.is_listening(),
        models_ready: models_ready(&models_dir()),
        phrases: cfg.phrases.clone(),
        hits: engine.hits(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

#[tauri::command]
fn set_phrases(phrases: Vec<String>, state: State<AppState>) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.phrases = phrases;
    save_config(&cfg).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn ensure_models(app: AppHandle) -> Result<(), String> {
    download_models(|p| {
        let _ = app.emit("model-progress", p);
    })?;
    Ok(())
}

#[tauri::command]
fn get_stats(
    start_ms: Option<i64>,
    end_ms: i64,
    phrase: Option<String>,
    bucket: String,
    tz_offset_min: i32,
) -> StatsResult {
    let hits = stats::load_hits();
    stats::query_stats(
        &hits,
        start_ms,
        end_ms,
        phrase.as_deref().filter(|s| !s.is_empty()),
        &bucket,
        tz_offset_min,
    )
}

#[tauri::command]
fn set_listening(listening: bool, app: AppHandle, state: State<AppState>) -> Result<(), String> {
    if listening {
        let cfg = state.config.lock().unwrap().clone();
        if !models_ready(&models_dir()) {
            return Err("models not downloaded".into());
        }
        let handle = app.clone();
        let notify = cfg.notify;
        state.engine.lock().unwrap().start(cfg, move |hit| {
            let _ = stats::append_hit(&hit);
            if notify {
                notify_macos(&hit);
            }
            let _ = handle.emit("hit", &hit);
        })?;
    } else {
        state.engine.lock().unwrap().stop();
    }
    let _ = app.emit("listening-changed", listening);
    Ok(())
}

#[tauri::command]
fn check_updates() -> String {
    let current = env!("CARGO_PKG_VERSION");
    match ureq::get("https://api.github.com/repos/jackaldenryan/phrase-watch/releases/latest")
        .set("User-Agent", "PhraseWatch")
        .call()
    {
        Ok(resp) => {
            let json: serde_json::Value = resp.into_json().unwrap_or_default();
            let tag = json
                .get("tag_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let url = json
                .get("html_url")
                .and_then(|v| v.as_str())
                .unwrap_or("https://github.com/jackaldenryan/phrase-watch/releases/latest");
            if tag.trim_start_matches('v') != current {
                format!("A newer version ({tag}) is available.\n{url}")
            } else {
                format!("PhraseWatch {current} is the latest release.")
            }
        }
        Err(_) => format!(
            "Could not check GitHub. Open https://github.com/jackaldenryan/phrase-watch/releases/latest"
        ),
    }
}

#[tauri::command]
fn process_wav_cmd(path: String, state: State<AppState>) -> Result<Vec<Hit>, String> {
    let cfg = state.config.lock().unwrap().clone();
    engine::process_wav(std::path::Path::new(&path), &cfg)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let cfg = load_config();
            app.manage(AppState {
                engine: Mutex::new(Engine::new()),
                config: Mutex::new(cfg),
            });

            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let listen = MenuItem::with_id(app, "listen", "Start/Stop Listening", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit PhraseWatch", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &listen, &quit])?;
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "listen" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            let listening = state.engine.lock().unwrap().is_listening();
                            let _ = set_listening(!listening, app.clone(), state);
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            set_phrases,
            ensure_models,
            set_listening,
            check_updates,
            process_wav_cmd,
            get_stats
        ])
        .run(tauri::generate_context!())
        .expect("error while running PhraseWatch");
}
