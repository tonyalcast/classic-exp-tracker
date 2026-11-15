mod actions;
mod detect;
mod hotkeys;
mod reader;
mod settings;
mod state;
mod storage;

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use exp_core::{ExpReading, ExpTable, SessionState};
use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

use state::AppState;
use storage::StoredSession;

const TICK: Duration = Duration::from_secs(1);

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> actions::View {
    actions::view(&state)
}

#[tauri::command]
fn session_toggle(app: AppHandle) -> Result<(), String> {
    actions::toggle_session(&app)
}

#[tauri::command]
fn session_stop(app: AppHandle) -> Result<Option<i64>, String> {
    actions::stop_session(&app)
}

#[tauri::command]
fn hud_toggle(app: AppHandle) -> Result<(), String> {
    actions::toggle_hud(&app)
}

#[tauri::command]
fn hud_set_locked(app: AppHandle, locked: bool) -> Result<(), String> {
    actions::set_hud_locked(&app, locked)
}

#[tauri::command]
fn hud_is_locked(state: State<'_, AppState>) -> bool {
    state.hud_locked.load(Ordering::Relaxed)
}

#[tauri::command]
fn list_sessions(state: State<'_, AppState>, limit: u32) -> Result<Vec<StoredSession>, String> {
    state.db.lock().unwrap().recent(limit).map_err(|e| e.to_string())
}

#[tauri::command]
fn reader_status(state: State<'_, AppState>) -> Option<reader::ReaderStatus> {
    state.reader_status.lock().unwrap().clone()
}

/// Binds a global hotkey; `accel` uses browser key codes, e.g. "Control+Shift+KeyE"; None restores the default.
#[tauri::command]
fn hotkey_set(app: AppHandle, action: settings::HotkeyAction, accel: Option<String>) -> Result<(), String> {
    hotkeys::bind(&app, action, accel.as_deref())
}

#[tauri::command]
fn hotkeys_reset(app: AppHandle) -> Result<(), String> {
    hotkeys::reset(&app)
}

/// Turns global hotkeys off while the main window records a new combo.
#[tauri::command]
fn hotkeys_suspend(app: AppHandle, suspend: bool) {
    hotkeys::suspend(&app, suspend);
}

/// Actions whose hotkey the OS refused to register.
#[tauri::command]
fn hotkeys_failed(state: State<'_, AppState>) -> Vec<settings::HotkeyAction> {
    state.hotkeys.failed()
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> settings::Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn update_settings(app: AppHandle, patch: settings::SettingsPatch) -> Result<settings::Settings, String> {
    actions::update_settings(&app, patch)
}

/// Asks the reader to read name and level again on the next tick.
#[tauri::command]
fn reader_identify(state: State<'_, AppState>) {
    state.identify_requested.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn capture_permission() -> bool {
    capture::permission::granted()
}

#[tauri::command]
fn capture_request_permission() -> bool {
    capture::permission::request()
}

#[tauri::command]
fn capture_open_settings() -> Result<(), String> {
    capture::permission::open_settings().map_err(|e| e.to_string())
}

#[tauri::command]
async fn capture_detect(app: AppHandle) -> Result<detect::Detection, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("captures");
    tauri::async_runtime::spawn_blocking(move || detect::detect(&dir))
        .await
        .map_err(|e| e.to_string())?
}

/// True in debug builds, where the dev feed is available.
#[tauri::command]
fn is_debug_build() -> bool {
    cfg!(debug_assertions)
}

/// Debug-only stand-in for the OCR pipeline.
#[tauri::command]
fn dev_push_reading(app: AppHandle, reading: ExpReading) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("only available in debug builds".into());
    }
    Ok(format!("{:?}", actions::ingest(&app, reading)))
}

/// Seconds between identity attempts while the game is not readable.
const IDENTIFY_RETRY: Duration = Duration::from_secs(5);

// Reads name and level once (retrying until it works), then only EXP while a session runs.
fn spawn_ticker(app: AppHandle, glyph_dir: PathBuf) {
    std::thread::spawn(move || {
        let mut reader = reader::Reader::new(glyph_dir, ExpTable::classic());
        let mut was_running = false;
        let mut next_identify = Instant::now();
        loop {
            std::thread::sleep(TICK);
            let st = app.state::<AppState>();
            if st.identify_requested.swap(false, Ordering::Relaxed) {
                reader.forget_identity();
                next_identify = Instant::now();
            }
            // An auto-paused session keeps reading so the next EXP gain resumes it.
            let running = {
                let s = st.session();
                s.state() == SessionState::Running || s.auto_paused()
            };
            if running && !was_running {
                reader.reset();
            }
            was_running = running;

            if reader.level().is_none() && (running || Instant::now() >= next_identify) {
                let status = match reader.identify() {
                    Ok((id, status)) => {
                        let mut s = st.session();
                        if id.name.is_some() {
                            s.set_character(id.name);
                        }
                        s.set_level_hint(Some(id.level));
                        status
                    }
                    Err(status) => {
                        next_identify = Instant::now() + IDENTIFY_RETRY;
                        status
                    }
                };
                report(&app, status);
                actions::publish(&app);
            }
            if !running || reader.level().is_none() {
                continue;
            }
            let (reading, status) = reader.step();
            if let Some(r) = reading {
                st.session().set_level_hint(Some(r.level));
                actions::ingest(&app, r);
            }
            if let Some(s) = status {
                report(&app, s);
            }
            st.session().tick(st.now());
            actions::publish(&app);
        }
    });
}

fn report(app: &AppHandle, status: reader::ReaderStatus) {
    *app.state::<AppState>().reader_status.lock().unwrap() = Some(status.clone());
    let _ = app.emit(actions::EV_READER, status);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(hotkeys::plugin())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = storage::Db::open(&dir.join("sessions.db"))?;
            app.manage(AppState::new(db, dir.clone()));
            let settings = app.state::<AppState>().settings.lock().unwrap().clone();
            if let Err(e) = actions::apply_hud_size(app.handle(), &settings) {
                eprintln!("could not apply HUD scale: {e}");
            }
            let keys = app.state::<AppState>().settings.lock().unwrap().hotkeys.clone();
            hotkeys::apply(app.handle(), &keys);
            spawn_ticker(app.handle().clone(), dir.join("glyphs"));
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window quits, the HUD has no close button.
            if window.label() == "main" && matches!(event, WindowEvent::CloseRequested { .. }) {
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            session_toggle,
            session_stop,
            hud_toggle,
            hud_set_locked,
            hud_is_locked,
            list_sessions,
            dev_push_reading,
            is_debug_build,
            capture_permission,
            capture_request_permission,
            capture_open_settings,
            capture_detect,
            reader_status,
            reader_identify,
            get_settings,
            hotkey_set,
            hotkeys_reset,
            hotkeys_suspend,
            hotkeys_failed,
            update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
