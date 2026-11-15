use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, Runtime, plugin::TauriPlugin};
use tauri_plugin_global_shortcut::{Builder, Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::actions;
use crate::settings::{HotkeyAction, Hotkeys};
use crate::state::AppState;

pub const EV_HOTKEYS_FAILED: &str = "hotkeys-failed";

/// Live registrations: shortcut id to action, plus actions the OS refused.
#[derive(Default)]
pub struct Registry {
    map: Mutex<HashMap<u32, HotkeyAction>>,
    failed: Mutex<Vec<HotkeyAction>>,
}

impl Registry {
    pub fn failed(&self) -> Vec<HotkeyAction> {
        self.failed.lock().unwrap().clone()
    }
}

/// Plugin that dispatches whatever shortcuts are currently registered.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let Some(st) = app.try_state::<AppState>() else { return };
            let Some(action) = st.hotkeys.map.lock().unwrap().get(&shortcut.id()).copied() else { return };
            let result = match action {
                HotkeyAction::Toggle => actions::toggle_session(app),
                HotkeyAction::Stop => actions::stop_session(app).map(|_| ()),
                HotkeyAction::Hud => actions::toggle_hud(app),
                HotkeyAction::Lock => actions::toggle_hud_lock(app),
            };
            if let Err(e) = result {
                eprintln!("hotkey {action:?} failed: {e}");
            }
        })
        .build()
}

/// Parses and normalizes an accelerator; needs Ctrl, Alt or Cmd/Win unless the key is F1–F24.
pub fn normalize(accel: &str) -> Result<String, String> {
    let s = Shortcut::from_str(accel).map_err(|_| format!("\"{accel}\" is not a valid shortcut"))?;
    let strong = Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER;
    if !s.mods.intersects(strong) && !is_function_key(s.key) {
        return Err("Use Ctrl, Alt or Cmd/Win, or a function key, so the game keeps its keys".into());
    }
    Ok(s.into_string())
}

fn is_function_key(code: Code) -> bool {
    use Code::*;
    matches!(
        code,
        F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12 | F13 | F14 | F15 | F16 | F17 | F18 | F19 | F20
            | F21 | F22 | F23 | F24
    )
}

/// Replaces every registration with `keys`; actions the OS refuses are reported, not fatal.
pub fn apply<R: Runtime>(app: &AppHandle<R>, keys: &Hotkeys) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let st = app.state::<AppState>();
    let mut map = st.hotkeys.map.lock().unwrap();
    map.clear();
    let mut failed = Vec::new();
    for action in HotkeyAction::ALL {
        match Shortcut::from_str(keys.get(action)) {
            Ok(s) if gs.register(s).is_ok() => {
                map.insert(s.id(), action);
            }
            _ => failed.push(action),
        }
    }
    drop(map);
    *st.hotkeys.failed.lock().unwrap() = failed.clone();
    let _ = app.emit(EV_HOTKEYS_FAILED, failed);
}

/// Drops all registrations so the main window can record a combo that is currently bound.
pub fn suspend<R: Runtime>(app: &AppHandle<R>, on: bool) {
    if on {
        let _ = app.global_shortcut().unregister_all();
        app.state::<AppState>().hotkeys.map.lock().unwrap().clear();
    } else {
        let keys = app.state::<AppState>().settings.lock().unwrap().hotkeys.clone();
        apply(app, &keys);
    }
}

/// Binds one action (None restores its default), rejecting duplicates and combos the OS refuses.
pub fn bind<R: Runtime>(app: &AppHandle<R>, action: HotkeyAction, accel: Option<&str>) -> Result<(), String> {
    let accel = match accel {
        Some(a) => normalize(a)?,
        None => Hotkeys::default().get(action).to_owned(),
    };
    let st = app.state::<AppState>();
    let mut keys = st.settings.lock().unwrap().hotkeys.clone();
    if let Some(other) = HotkeyAction::ALL.into_iter().find(|a| *a != action && keys.get(*a) == accel) {
        return Err(format!("Already used by \"{}\"", other.label()));
    }
    let previous = keys.get(action).to_owned();
    keys.set(action, accel.clone());
    apply(app, &keys);
    if st.hotkeys.failed().contains(&action) {
        keys.set(action, previous);
        apply(app, &keys);
        return Err("Taken by the system or another app; pick another combo".into());
    }
    commit(app, keys)
}

/// Restores every default binding.
pub fn reset<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let keys = Hotkeys::default();
    apply(app, &keys);
    commit(app, keys)
}

fn commit<R: Runtime>(app: &AppHandle<R>, keys: Hotkeys) -> Result<(), String> {
    let st = app.state::<AppState>();
    let settings = {
        let mut s = st.settings.lock().unwrap();
        s.hotkeys = keys;
        s.clone()
    };
    actions::save_and_emit(app, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_browser_codes() {
        assert_eq!(normalize("Control+Shift+F9").unwrap(), "shift+control+F9");
        assert_eq!(normalize("Alt+KeyE").unwrap(), "alt+KeyE");
        assert_eq!(normalize("Super+Digit1").unwrap(), "super+Digit1");
        assert_eq!(normalize("F8").unwrap(), "F8");
        assert_eq!(normalize("Shift+F2").unwrap(), "shift+F2");
    }

    #[test]
    fn rejects_combos_that_steal_game_keys() {
        assert!(normalize("KeyA").is_err());
        assert!(normalize("Shift+KeyA").is_err());
        assert!(normalize("Control+Nope").is_err());
    }

    #[test]
    fn defaults_are_valid_and_normalized() {
        let d = Hotkeys::default();
        for a in HotkeyAction::ALL {
            assert_eq!(normalize(d.get(a)).unwrap(), d.get(a));
        }
    }
}
