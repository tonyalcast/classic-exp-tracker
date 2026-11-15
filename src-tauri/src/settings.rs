use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 100% is the smallest HUD; it only grows.
pub const HUD_SCALE_MIN: f64 = 1.0;
pub const HUD_SCALE_MAX: f64 = 2.0;
/// HUD background opacity range; text always stays opaque.
pub const HUD_OPACITY_MIN: f64 = 0.3;
pub const HUD_OPACITY_MAX: f64 = 1.0;
/// Longest idle auto-pause delay; 0 turns it off.
pub const IDLE_PAUSE_MAX_MINS: u32 = 30;
/// HUD window size at 100%, matching tauri.conf.json; the chart row adds height.
pub const HUD_BASE_W: f64 = 250.0;
pub const HUD_BASE_H: f64 = 105.0;
pub const HUD_CHART_H: f64 = 123.0;

/// Which rate drives the headline numbers and time to level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum RateWindow {
    #[serde(rename = "1m")]
    OneMin,
    #[default]
    #[serde(rename = "5m")]
    FiveMin,
    #[serde(rename = "15m")]
    FifteenMin,
    #[serde(rename = "session")]
    Session,
}

/// Actions that can be bound to a global hotkey.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum HotkeyAction {
    Toggle,
    Stop,
    Hud,
    Lock,
}

impl HotkeyAction {
    pub const ALL: [HotkeyAction; 4] = [Self::Toggle, Self::Stop, Self::Hud, Self::Lock];

    pub fn label(self) -> &'static str {
        match self {
            Self::Toggle => "Start / pause",
            Self::Stop => "Stop & save",
            Self::Hud => "Show / hide HUD",
            Self::Lock => "Lock / unlock HUD",
        }
    }
}

/// Global hotkeys in normalized accelerator form, e.g. "shift+control+F9".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Hotkeys {
    pub toggle: String,
    pub stop: String,
    pub hud: String,
    pub lock: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            toggle: "shift+control+F9".into(),
            stop: "shift+control+F10".into(),
            hud: "shift+control+F11".into(),
            lock: "shift+control+F12".into(),
        }
    }
}

impl Hotkeys {
    pub fn get(&self, a: HotkeyAction) -> &str {
        match a {
            HotkeyAction::Toggle => &self.toggle,
            HotkeyAction::Stop => &self.stop,
            HotkeyAction::Hud => &self.hud,
            HotkeyAction::Lock => &self.lock,
        }
    }

    pub fn set(&mut self, a: HotkeyAction, accel: String) {
        let slot = match a {
            HotkeyAction::Toggle => &mut self.toggle,
            HotkeyAction::Stop => &mut self.stop,
            HotkeyAction::Hud => &mut self.hud,
            HotkeyAction::Lock => &mut self.lock,
        };
        *slot = accel;
    }
}

/// User preferences persisted as JSON in the app-data directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub hud_scale: f64,
    pub hud_opacity: f64,
    pub rate_window: RateWindow,
    pub idle_pause_mins: u32,
    pub hud_chart: bool,
    pub hotkeys: Hotkeys,
}

/// Partial update sent by the UI; missing fields keep their value.
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub hud_scale: Option<f64>,
    pub hud_opacity: Option<f64>,
    pub rate_window: Option<RateWindow>,
    pub idle_pause_mins: Option<u32>,
    pub hud_chart: Option<bool>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hud_scale: 1.0,
            hud_opacity: 0.9,
            rate_window: RateWindow::FiveMin,
            idle_pause_mins: 3,
            hud_chart: true,
            hotkeys: Hotkeys::default(),
        }
    }
}

impl Settings {
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("settings.json")
    }

    /// Loads settings, falling back to defaults on a missing or broken file.
    pub fn load(dir: &Path) -> Self {
        std::fs::read_to_string(Self::path(dir))
            .ok()
            .and_then(|t| serde_json::from_str::<Self>(&t).ok())
            .unwrap_or_default()
            .clamped()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(Self::path(dir), json)
    }

    pub fn clamped(mut self) -> Self {
        self.hud_scale = clamp_or(self.hud_scale, HUD_SCALE_MIN, HUD_SCALE_MAX, 1.0);
        self.hud_opacity = clamp_or(self.hud_opacity, HUD_OPACITY_MIN, HUD_OPACITY_MAX, 0.9);
        self.idle_pause_mins = self.idle_pause_mins.min(IDLE_PAUSE_MAX_MINS);
        self
    }

    /// Applies a patch and returns the clamped result.
    pub fn merged(self, p: SettingsPatch) -> Self {
        Self {
            hud_scale: p.hud_scale.unwrap_or(self.hud_scale),
            hud_opacity: p.hud_opacity.unwrap_or(self.hud_opacity),
            rate_window: p.rate_window.unwrap_or(self.rate_window),
            idle_pause_mins: p.idle_pause_mins.unwrap_or(self.idle_pause_mins),
            hud_chart: p.hud_chart.unwrap_or(self.hud_chart),
            hotkeys: self.hotkeys,
        }
        .clamped()
    }

    /// HUD window size in logical px for the current scale and chart row.
    pub fn hud_size(&self) -> (f64, f64) {
        let h = if self.hud_chart { HUD_CHART_H } else { HUD_BASE_H };
        (HUD_BASE_W * self.hud_scale, h * self.hud_scale)
    }

    /// Idle auto-pause delay in seconds, None when off.
    pub fn idle_limit_secs(&self) -> Option<f64> {
        (self.idle_pause_mins > 0).then_some(self.idle_pause_mins as f64 * 60.0)
    }
}

fn clamp_or(v: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if v.is_finite() { v.clamp(min, max) } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_clamp() {
        let dir = std::env::temp_dir().join(format!("cet-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(Settings::load(&dir), Settings::default());
        let s = Settings { hud_scale: 1.4, rate_window: RateWindow::Session, ..Settings::default() };
        s.save(&dir).unwrap();
        assert_eq!(Settings::load(&dir), s);
        Settings { hud_scale: 9.0, hud_opacity: 0.0, idle_pause_mins: 99, ..s }.save(&dir).unwrap();
        let back = Settings::load(&dir);
        assert_eq!(back.hud_scale, HUD_SCALE_MAX);
        assert_eq!(back.hud_opacity, HUD_OPACITY_MIN);
        assert_eq!(back.idle_pause_mins, IDLE_PAUSE_MAX_MINS);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn old_file_gets_new_defaults_and_patch_merges() {
        let s: Settings = serde_json::from_str(r#"{"hudScale":1.2}"#).unwrap();
        assert_eq!(s.hud_scale, 1.2);
        assert_eq!(s.idle_pause_mins, 3);
        assert_eq!(s.hotkeys, Hotkeys::default());
        assert!(s.hud_chart);
        assert_eq!(Settings { hud_chart: false, hud_scale: 2.0, ..s.clone() }.hud_size(), (500.0, 210.0));
        let p: SettingsPatch = serde_json::from_str(r#"{"rateWindow":"15m","idlePauseMins":0}"#).unwrap();
        let m = s.merged(p);
        assert_eq!(m.rate_window, RateWindow::FifteenMin);
        assert_eq!(m.idle_limit_secs(), None);
        assert_eq!(m.hud_scale, 1.2);
    }
}
