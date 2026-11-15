use std::sync::atomic::Ordering;

use exp_core::{ExpReading, Ingest, SessionState, Snapshot};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::settings::{Settings, SettingsPatch};
use crate::state::{AppState, Gain, RECENT_GAINS, unix_ms};

pub const EV_SNAPSHOT: &str = "snapshot";
pub const EV_HUD_LOCKED: &str = "hud-locked";
pub const EV_HISTORY: &str = "history-changed";
pub const EV_READER: &str = "reader-status";
pub const EV_SETTINGS: &str = "settings";
pub const HUD: &str = "hud";
/// Shortest saved session that counts as a best run.
const BEST_MIN_SECS: f64 = 300.0;

/// Session snapshot plus app-level extras, as sent to the UI.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    #[serde(flatten)]
    pub snap: Snapshot,
    /// Newest first.
    pub recent_gains: Vec<Gain>,
}

pub fn view(st: &AppState) -> View {
    refresh_best(st);
    let snap = st.session().snapshot(st.now());
    let recent_gains = st.gains.lock().unwrap().iter().copied().collect();
    View { snap, recent_gains }
}

/// Emits the current snapshot to every window.
pub fn publish<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(EV_SNAPSHOT, view(&app.state::<AppState>()));
}

// Looks up the best saved rate again only when level or character changed.
fn refresh_best(st: &AppState) {
    let key = {
        let s = st.session();
        s.level().map(|l| (l, s.character().map(str::to_owned)))
    };
    let mut cached = st.best_key.lock().unwrap();
    if key.is_none() || *cached == key {
        return;
    }
    let rate = key.as_ref().and_then(|(level, who)| {
        st.db.lock().unwrap().best_rate(*level, who.as_deref(), BEST_MIN_SECS).ok().flatten()
    });
    st.session().set_best_rate(rate);
    *cached = key;
}

pub fn toggle_session<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let st = app.state::<AppState>();
    {
        let mut s = st.session();
        if s.state() == SessionState::Idle {
            *st.started_at_ms.lock().unwrap() = Some(unix_ms());
            st.gains.lock().unwrap().clear();
        }
        s.toggle(st.now());
    }
    publish(app);
    Ok(())
}

/// Stops the session and stores it; returns the new row id when something was saved.
pub fn stop_session<R: Runtime>(app: &AppHandle<R>) -> Result<Option<i64>, String> {
    let st = app.state::<AppState>();
    let summary = st.session().stop(st.now());
    let started = st.started_at_ms.lock().unwrap().take();
    let saved = match (summary, started) {
        (Some(sum), Some(start)) if sum.active_secs > 0.0 => Some(
            st.db
                .lock()
                .unwrap()
                .insert(start, unix_ms(), &sum)
                .map_err(|e| e.to_string())?,
        ),
        _ => None,
    };
    if saved.is_some() {
        // The new row may be the new best.
        *st.best_key.lock().unwrap() = None;
    }
    publish(app);
    if saved.is_some() {
        let _ = app.emit(EV_HISTORY, ());
    }
    Ok(saved)
}

pub fn ingest<R: Runtime>(app: &AppHandle<R>, reading: ExpReading) -> Option<Ingest> {
    let st = app.state::<AppState>();
    let out = st.session().ingest(reading, st.now());
    if let Some(Ingest::Gained { exp, .. }) = out {
        let mut gains = st.gains.lock().unwrap();
        gains.push_front(Gain { at_ms: unix_ms(), exp });
        gains.truncate(RECENT_GAINS);
    }
    if out.is_some() {
        publish(app);
    }
    out
}

pub fn toggle_hud<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let hud = app.get_webview_window(HUD).ok_or("hud window missing")?;
    let visible = hud.is_visible().map_err(|e| e.to_string())?;
    if visible { hud.hide() } else { hud.show() }.map_err(|e| e.to_string())
}

pub fn set_hud_locked<R: Runtime>(app: &AppHandle<R>, locked: bool) -> Result<(), String> {
    let hud = app.get_webview_window(HUD).ok_or("hud window missing")?;
    hud.set_ignore_cursor_events(locked).map_err(|e| e.to_string())?;
    app.state::<AppState>().hud_locked.store(locked, Ordering::Relaxed);
    let _ = app.emit(EV_HUD_LOCKED, locked);
    Ok(())
}

pub fn toggle_hud_lock<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let locked = app.state::<AppState>().hud_locked.load(Ordering::Relaxed);
    set_hud_locked(app, !locked)
}

/// Resizes the HUD window to fit its scale and chart row; the page zooms its content.
pub fn apply_hud_size<R: Runtime>(app: &AppHandle<R>, s: &Settings) -> Result<(), String> {
    let hud = app.get_webview_window(HUD).ok_or("hud window missing")?;
    let (w, h) = s.hud_size();
    hud.set_size(tauri::LogicalSize::new(w, h)).map_err(|e| e.to_string())
}

/// Merges, persists and applies a settings change, then tells the windows.
pub fn update_settings<R: Runtime>(app: &AppHandle<R>, patch: SettingsPatch) -> Result<Settings, String> {
    let st = app.state::<AppState>();
    let (old, new) = {
        let mut s = st.settings.lock().unwrap();
        let old = s.clone();
        *s = old.clone().merged(patch);
        (old, s.clone())
    };
    if new.hud_size() != old.hud_size() {
        apply_hud_size(app, &new)?;
    }
    if new.idle_pause_mins != old.idle_pause_mins {
        st.session().set_idle_limit(new.idle_limit_secs());
    }
    save_and_emit(app, &new)?;
    Ok(new)
}

/// Persists settings and pushes them to both windows.
pub fn save_and_emit<R: Runtime>(app: &AppHandle<R>, s: &Settings) -> Result<(), String> {
    s.save(&app.state::<AppState>().data_dir).map_err(|e| e.to_string())?;
    let _ = app.emit(EV_SETTINGS, s);
    Ok(())
}
