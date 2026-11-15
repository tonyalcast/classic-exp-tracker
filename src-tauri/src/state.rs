use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, MutexGuard};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use exp_core::{ExpTable, Session};
use serde::Serialize;

use crate::hotkeys::Registry;
use crate::reader::ReaderStatus;
use crate::settings::Settings;
use crate::storage::Db;

/// Shared backend state managed by Tauri.
pub struct AppState {
    epoch: Instant,
    session: Mutex<Session>,
    pub started_at_ms: Mutex<Option<i64>>,
    pub db: Mutex<Db>,
    pub hud_locked: AtomicBool,
    pub reader_status: Mutex<Option<ReaderStatus>>,
    pub identify_requested: AtomicBool,
    pub settings: Mutex<Settings>,
    pub data_dir: PathBuf,
    // (level, character) the cached best rate was looked up for; None forces a lookup.
    pub best_key: Mutex<Option<BestKey>>,
    pub hotkeys: Registry,
    pub gains: Mutex<VecDeque<Gain>>,
}

/// Most recent EXP gains kept for the gain feed.
pub const RECENT_GAINS: usize = 5;

/// One accepted EXP gain, with wall-clock time.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gain {
    pub at_ms: i64,
    pub exp: u64,
}

pub type BestKey = (u32, Option<String>);

impl AppState {
    pub fn new(db: Db, data_dir: PathBuf) -> Self {
        let settings = Settings::load(&data_dir);
        let mut session = Session::new(Some(ExpTable::classic()));
        session.set_idle_limit(settings.idle_limit_secs());
        Self {
            epoch: Instant::now(),
            session: Mutex::new(session),
            started_at_ms: Mutex::new(None),
            db: Mutex::new(db),
            hud_locked: AtomicBool::new(false),
            reader_status: Mutex::new(None),
            identify_requested: AtomicBool::new(false),
            settings: Mutex::new(settings),
            data_dir,
            best_key: Mutex::new(None),
            hotkeys: Registry::default(),
            gains: Mutex::new(VecDeque::with_capacity(RECENT_GAINS + 1)),
        }
    }

    /// Monotonic seconds since app start.
    pub fn now(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    pub fn session(&self) -> MutexGuard<'_, Session> {
        self.session.lock().unwrap()
    }
}

pub fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
