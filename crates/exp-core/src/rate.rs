use std::collections::VecDeque;

/// Minimum active time before a rate is reported, to avoid noisy early numbers.
pub const MIN_ELAPSED_SECS: f64 = 5.0;

/// EXP-per-second over sliding windows of active session time.
#[derive(Debug, Clone)]
pub struct RollingRate {
    // (active seconds, cumulative exp gained), oldest first.
    samples: VecDeque<(f64, u64)>,
    max_window: f64,
}

impl RollingRate {
    pub fn new(max_window_secs: f64) -> Self {
        Self { samples: VecDeque::new(), max_window: max_window_secs }
    }

    pub fn push(&mut self, t: f64, cumulative: u64) {
        if let Some(&(lt, lc)) = self.samples.back()
            && lt == t
        {
            self.samples.pop_back();
            self.samples.push_back((t, cumulative.max(lc)));
            return;
        }
        self.samples.push_back((t, cumulative));
        // Keep one sample older than the window as the window's left edge.
        while self.samples.len() > 2 && self.samples[1].0 <= t - self.max_window {
            self.samples.pop_front();
        }
    }

    /// EXP per second over the last `window` seconds, or since the start if shorter.
    pub fn per_sec(&self, window: f64) -> Option<f64> {
        let &(now, cum_now) = self.samples.back()?;
        let edge = now - window;
        let &(t0, c0) = self
            .samples
            .iter()
            .rev()
            .find(|(t, _)| *t <= edge)
            .unwrap_or(self.samples.front()?);
        let span = now - t0;
        (span >= MIN_ELAPSED_SECS).then(|| (cum_now - c0) as f64 / span)
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// Drops samples taken after active time `t`.
    pub fn truncate_after(&mut self, t: f64) {
        while self.samples.back().is_some_and(|&(st, _)| st > t) {
            self.samples.pop_back();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steady_rate() {
        let mut r = RollingRate::new(900.0);
        for s in 0..=600 {
            r.push(s as f64, s as u64 * 10);
        }
        assert_eq!(r.per_sec(60.0), Some(10.0));
        assert_eq!(r.per_sec(300.0), Some(10.0));
    }

    #[test]
    fn window_reflects_recent_change() {
        let mut r = RollingRate::new(900.0);
        for s in 0..=300 {
            r.push(s as f64, 0);
        }
        for s in 301..=360 {
            r.push(s as f64, (s - 300) as u64 * 20);
        }
        assert_eq!(r.per_sec(60.0), Some(20.0));
        assert!(r.per_sec(300.0).unwrap() < 5.0);
    }

    #[test]
    fn short_session_uses_elapsed_and_needs_min_time() {
        let mut r = RollingRate::new(900.0);
        r.push(0.0, 0);
        r.push(2.0, 100);
        assert_eq!(r.per_sec(60.0), None);
        r.push(10.0, 500);
        assert_eq!(r.per_sec(60.0), Some(50.0));
    }

    #[test]
    fn prunes_old_samples() {
        let mut r = RollingRate::new(10.0);
        for s in 0..1000 {
            r.push(s as f64, s as u64);
        }
        assert!(r.samples.len() <= 12);
        assert_eq!(r.per_sec(10.0), Some(1.0));
    }
}
