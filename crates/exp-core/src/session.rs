use std::collections::VecDeque;

use serde::Serialize;

use crate::{ExpReading, ExpTable, ExpTracker, Ingest, RollingRate};

const WINDOW_1M: f64 = 60.0;
const WINDOW_5M: f64 = 300.0;
const WINDOW_15M: f64 = 900.0;
/// Active seconds between trend samples.
pub const TREND_STEP_SECS: f64 = 20.0;
/// Trend samples kept: 90 x 20 s = the last 30 active minutes.
pub const TREND_LEN: usize = 90;
/// Active time needed before the best-session comparison is shown.
const VS_BEST_MIN_SECS: f64 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Idle,
    Running,
    Paused,
}

/// Everything the UI needs to render, in one serializable value.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub state: SessionState,
    /// True when the session paused itself after no EXP gain.
    pub auto_paused: bool,
    pub character: Option<String>,
    pub level: Option<u32>,
    pub exp: Option<u64>,
    pub percent: Option<f64>,
    pub exp_gained: u64,
    pub levels_gained: u32,
    pub active_secs: f64,
    /// Rates in EXP per second; None until enough time has passed.
    pub rate_session: Option<f64>,
    pub rate_1m: Option<f64>,
    pub rate_5m: Option<f64>,
    pub rate_15m: Option<f64>,
    pub exp_to_level: Option<u64>,
    pub secs_to_level: Option<f64>,
    /// 1-minute rate sampled every TREND_STEP_SECS of active time, oldest first.
    pub trend: Vec<f64>,
    /// Best saved session rate for this level, EXP per second.
    pub best_rate: Option<f64>,
    /// Seconds ahead (positive) or behind (negative) the best session's pace.
    pub vs_best_secs: Option<f64>,
}

/// Final numbers of a stopped session, ready to persist.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub character: Option<String>,
    pub start_level: Option<u32>,
    pub end_level: Option<u32>,
    pub exp_gained: u64,
    pub levels_gained: u32,
    pub active_secs: f64,
}

/// Manually controlled training session; `now` is any monotonic clock in seconds.
#[derive(Debug)]
pub struct Session {
    state: SessionState,
    table: Option<ExpTable>,
    tracker: ExpTracker,
    rate: RollingRate,
    character: Option<String>,
    // Level known from outside a session, shown until readings arrive.
    level_hint: Option<u32>,
    start_level: Option<u32>,
    // Active seconds banked before the current running stretch.
    banked: f64,
    running_since: Option<f64>,
    // (active secs, 1-minute rate) samples for the sparkline.
    trend: VecDeque<(f64, f64)>,
    // Pause after this many active seconds without an EXP gain.
    idle_limit: Option<f64>,
    auto_paused: bool,
    last_gain_at: f64,
    last_gained: u64,
    best_rate: Option<f64>,
}

impl Session {
    pub fn new(table: Option<ExpTable>) -> Self {
        Self {
            state: SessionState::Idle,
            tracker: ExpTracker::new(table.clone()),
            table,
            rate: RollingRate::new(WINDOW_15M),
            character: None,
            level_hint: None,
            start_level: None,
            banked: 0.0,
            running_since: None,
            trend: VecDeque::with_capacity(TREND_LEN + 1),
            idle_limit: None,
            auto_paused: false,
            last_gain_at: 0.0,
            last_gained: 0,
            best_rate: None,
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn auto_paused(&self) -> bool {
        self.auto_paused
    }

    /// Last accepted reading.
    pub fn last_reading(&self) -> Option<ExpReading> {
        self.tracker.last()
    }

    /// Current level from readings, else the identity hint.
    pub fn level(&self) -> Option<u32> {
        self.tracker.last().map(|r| r.level).or(self.level_hint)
    }

    pub fn character(&self) -> Option<&str> {
        self.character.as_deref()
    }

    /// EXP required for the current level, from the table or estimated.
    pub fn required_estimate(&self) -> Option<u64> {
        self.tracker.last().and_then(|r| self.tracker.required(r.level))
    }

    pub fn set_character(&mut self, name: Option<String>) {
        self.character = name;
    }

    pub fn set_level_hint(&mut self, level: Option<u32>) {
        self.level_hint = level;
    }

    /// Auto-pause after `secs` of active time without EXP; None disables it.
    pub fn set_idle_limit(&mut self, secs: Option<f64>) {
        self.idle_limit = secs.filter(|s| *s > 0.0);
    }

    /// Best saved rate (EXP per second) to compare the session against.
    pub fn set_best_rate(&mut self, rate: Option<f64>) {
        self.best_rate = rate.filter(|r| *r > 0.0);
    }

    pub fn active_secs(&self, now: f64) -> f64 {
        self.banked + self.running_since.map_or(0.0, |s| (now - s).max(0.0))
    }

    /// Starts a fresh session from idle; no-op otherwise.
    pub fn start(&mut self, now: f64) {
        if self.state != SessionState::Idle {
            return;
        }
        self.tracker = ExpTracker::new(self.table.clone());
        self.rate.clear();
        self.trend.clear();
        self.start_level = None;
        self.banked = 0.0;
        self.last_gain_at = 0.0;
        self.last_gained = 0;
        self.auto_paused = false;
        self.running_since = Some(now);
        self.state = SessionState::Running;
        self.rate.push(0.0, 0);
    }

    pub fn pause(&mut self, now: f64) {
        if self.state == SessionState::Running {
            self.banked = self.active_secs(now);
            self.running_since = None;
            self.state = SessionState::Paused;
        }
        self.auto_paused = false;
    }

    pub fn resume(&mut self, now: f64) {
        if self.state == SessionState::Paused {
            self.running_since = Some(now);
            self.state = SessionState::Running;
        }
        self.auto_paused = false;
    }

    /// Idle starts, running pauses, paused resumes.
    pub fn toggle(&mut self, now: f64) {
        match self.state {
            SessionState::Idle => self.start(now),
            SessionState::Running => self.pause(now),
            SessionState::Paused => self.resume(now),
        }
    }

    /// Ends the session and returns its summary, or None when it was idle.
    pub fn stop(&mut self, now: f64) -> Option<SessionSummary> {
        if self.state == SessionState::Idle {
            return None;
        }
        let summary = SessionSummary {
            character: self.character.clone(),
            start_level: self.start_level,
            end_level: self.tracker.last().map(|r| r.level),
            exp_gained: self.tracker.gained(),
            levels_gained: self.tracker.levels_gained(),
            active_secs: self.active_secs(now),
        };
        self.banked = summary.active_secs;
        self.running_since = None;
        self.auto_paused = false;
        self.state = SessionState::Idle;
        Some(summary)
    }

    /// Feeds a HUD reading; ignored unless running, and a changed reading wakes an auto-pause.
    pub fn ingest(&mut self, reading: ExpReading, now: f64) -> Option<Ingest> {
        let wake = self.auto_paused
            && self.tracker.last().is_some_and(|l| (l.level, l.exp) != (reading.level, reading.exp));
        if wake {
            self.resume(now);
        }
        if self.state != SessionState::Running {
            return None;
        }
        let out = self.tracker.feed(reading);
        if wake && matches!(out, Ingest::Rejected(_)) {
            // A misread must not end the auto-pause.
            self.pause(now);
            self.auto_paused = true;
            return Some(out);
        }
        if self.start_level.is_none() && !matches!(out, Ingest::Rejected(_)) {
            self.start_level = Some(reading.level);
        }
        let gained = self.tracker.gained();
        if gained > self.last_gained {
            self.last_gained = gained;
            self.last_gain_at = self.active_secs(now);
        }
        self.tick(now);
        Some(out)
    }

    /// Records rate and trend samples and applies the idle limit; call periodically.
    pub fn tick(&mut self, now: f64) {
        if self.state != SessionState::Running {
            return;
        }
        let t = self.active_secs(now);
        if self.idle_limit.is_some_and(|limit| t - self.last_gain_at >= limit) {
            self.auto_pause();
            return;
        }
        self.rate.push(t, self.tracker.gained());
        let due = self.trend.back().map_or(t >= TREND_STEP_SECS, |&(bt, _)| t - bt >= TREND_STEP_SECS);
        if due {
            self.trend.push_back((t, self.rate.per_sec(WINDOW_1M).unwrap_or(0.0)));
            if self.trend.len() > TREND_LEN {
                self.trend.pop_front();
            }
        }
    }

    // Pauses and gives back the idle stretch so it does not dilute the rates.
    fn auto_pause(&mut self) {
        let t = self.last_gain_at;
        self.banked = t;
        self.running_since = None;
        self.state = SessionState::Paused;
        self.auto_paused = true;
        self.rate.truncate_after(t);
        while self.trend.back().is_some_and(|&(bt, _)| bt > t) {
            self.trend.pop_back();
        }
    }

    pub fn snapshot(&self, now: f64) -> Snapshot {
        let active = self.active_secs(now);
        let last = self.tracker.last();
        let gained = self.tracker.gained();
        let rate_session = (active >= crate::rate::MIN_ELAPSED_SECS).then(|| gained as f64 / active);
        let rate_5m = self.rate.per_sec(WINDOW_5M);
        let exp_to_level = self.tracker.exp_to_next_level();
        let secs_to_level = exp_to_level.zip(rate_5m.or(rate_session)).and_then(|(left, r)| {
            (r > 0.0).then(|| left as f64 / r)
        });
        let vs_best_secs = self
            .best_rate
            .filter(|_| self.state != SessionState::Idle && active >= VS_BEST_MIN_SECS)
            .map(|best| gained as f64 / best - active);
        Snapshot {
            state: self.state,
            auto_paused: self.auto_paused,
            character: self.character.clone(),
            level: self.level(),
            exp: last.map(|r| r.exp),
            percent: last.map(|r| r.percent),
            exp_gained: gained,
            levels_gained: self.tracker.levels_gained(),
            active_secs: active,
            rate_session,
            rate_1m: self.rate.per_sec(WINDOW_1M),
            rate_5m,
            rate_15m: self.rate.per_sec(WINDOW_15M),
            exp_to_level,
            secs_to_level,
            trend: self.trend.iter().map(|&(_, r)| r).collect(),
            best_rate: self.best_rate,
            vs_best_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_time_excludes_pauses() {
        let mut s = Session::new(None);
        s.start(100.0);
        s.pause(160.0);
        s.resume(500.0);
        assert_eq!(s.active_secs(530.0), 90.0);
        let sum = s.stop(530.0).unwrap();
        assert_eq!(sum.active_secs, 90.0);
        assert_eq!(s.state(), SessionState::Idle);
    }

    #[test]
    fn ignores_readings_unless_running() {
        let mut s = Session::new(None);
        assert_eq!(s.ingest(ExpReading::new(10, 100, 10.0), 0.0), None);
        s.start(0.0);
        s.ingest(ExpReading::new(10, 100, 10.0), 1.0);
        s.pause(2.0);
        assert_eq!(s.ingest(ExpReading::new(10, 900, 90.0), 3.0), None);
        assert_eq!(s.snapshot(3.0).exp_gained, 0);
    }

    #[test]
    fn snapshot_rates_and_time_to_level() {
        let mut s = Session::new(None);
        s.start(0.0);
        s.ingest(ExpReading::new(20, 0, 0.0), 0.0);
        s.ingest(ExpReading::new(20, 1_000, 10.0), 0.5);
        for t in 1..=100 {
            s.ingest(ExpReading::new(20, 1_000 + t * 10, 10.0 + t as f64 * 0.1), t as f64);
        }
        let snap = s.snapshot(100.0);
        assert_eq!(snap.exp_gained, 2_000);
        assert_eq!(snap.rate_session, Some(20.0));
        assert_eq!(snap.rate_1m, Some(10.0));
        assert_eq!(snap.exp_to_level, Some(8_000));
        assert!(snap.secs_to_level.is_some());
    }

    #[test]
    fn restart_resets_counters() {
        let mut s = Session::new(None);
        s.start(0.0);
        s.ingest(ExpReading::new(5, 10, 10.0), 0.0);
        s.ingest(ExpReading::new(5, 50, 50.0), 10.0);
        s.stop(10.0);
        s.start(20.0);
        let snap = s.snapshot(20.0);
        assert_eq!(snap.exp_gained, 0);
        assert_eq!(snap.active_secs, 0.0);
        assert!(snap.trend.is_empty());
    }

    #[test]
    fn trend_samples_every_step_and_is_capped() {
        let mut s = Session::new(None);
        s.start(0.0);
        for t in 0..=3_000u64 {
            s.ingest(ExpReading::new(20, t * 2, 1.0), t as f64);
        }
        let snap = s.snapshot(3_000.0);
        assert_eq!(snap.trend.len(), TREND_LEN);
        assert!(snap.trend.iter().all(|r| (*r - 2.0).abs() < 1e-9));
    }

    #[test]
    fn idle_limit_pauses_and_returns_idle_time() {
        let mut s = Session::new(None);
        s.set_idle_limit(Some(120.0));
        s.start(0.0);
        s.ingest(ExpReading::new(20, 0, 0.0), 0.0);
        s.ingest(ExpReading::new(20, 600, 6.0), 60.0);
        for t in 61..=200 {
            s.tick(t as f64);
        }
        assert_eq!(s.state(), SessionState::Paused);
        assert!(s.auto_paused());
        let snap = s.snapshot(200.0);
        assert_eq!(snap.active_secs, 60.0);
        assert_eq!(snap.rate_session, Some(10.0));
    }

    #[test]
    fn changed_reading_wakes_auto_pause() {
        let mut s = Session::new(None);
        s.set_idle_limit(Some(30.0));
        s.start(0.0);
        s.ingest(ExpReading::new(20, 0, 0.0), 0.0);
        s.ingest(ExpReading::new(20, 100, 1.0), 10.0);
        s.tick(50.0);
        assert!(s.auto_paused());
        assert_eq!(s.ingest(ExpReading::new(20, 100, 1.0), 60.0), None);
        assert!(s.auto_paused());
        s.ingest(ExpReading::new(20, 150, 1.5), 300.0);
        assert_eq!(s.state(), SessionState::Running);
        assert!(!s.auto_paused());
        assert_eq!(s.snapshot(300.0).exp_gained, 150);
        assert_eq!(s.active_secs(310.0), 20.0);
    }

    #[test]
    fn manual_pause_is_not_woken() {
        let mut s = Session::new(None);
        s.start(0.0);
        s.ingest(ExpReading::new(20, 0, 0.0), 0.0);
        s.pause(5.0);
        assert_eq!(s.ingest(ExpReading::new(20, 100, 1.0), 6.0), None);
        assert_eq!(s.state(), SessionState::Paused);
    }

    #[test]
    fn vs_best_compares_pace() {
        let mut s = Session::new(None);
        s.set_best_rate(Some(10.0));
        s.start(0.0);
        s.ingest(ExpReading::new(20, 0, 0.0), 0.0);
        s.ingest(ExpReading::new(20, 1_200, 12.0), 100.0);
        // 1,200 EXP takes the best pace 120 s, done in 100 s: 20 s ahead.
        assert_eq!(s.snapshot(100.0).vs_best_secs, Some(20.0));
        assert_eq!(s.snapshot(30.0).vs_best_secs, None);
    }
}
