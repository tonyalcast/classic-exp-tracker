use crate::{ExpReading, ExpTable};

/// Consecutive rejections after which the tracker re-baselines on the latest reading.
const RESYNC_AFTER: u32 = 3;

/// Why a reading was discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reject {
    PercentOutOfRange,
    ExpDecreased,
    LevelDecreased,
    Inconsistent,
    UnknownRequired,
}

/// Outcome of feeding one reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ingest {
    Baseline,
    Gained { exp: u64, levels: u32 },
    Rejected(Reject),
    Resynced,
}

/// Turns a stream of HUD readings into accumulated EXP gain.
#[derive(Debug, Default)]
pub struct ExpTracker {
    table: Option<ExpTable>,
    last: Option<ExpReading>,
    // Reading at the current level with the highest percent, the most precise source for estimates.
    best_ref: Option<ExpReading>,
    gained: u64,
    levels_gained: u32,
    rejects_in_row: u32,
}

impl ExpTracker {
    pub fn new(table: Option<ExpTable>) -> Self {
        Self { table, ..Default::default() }
    }

    pub fn gained(&self) -> u64 {
        self.gained
    }

    pub fn levels_gained(&self) -> u32 {
        self.levels_gained
    }

    pub fn last(&self) -> Option<ExpReading> {
        self.last
    }

    /// EXP required for `level`, from the table or estimated from HUD readings.
    pub fn required(&self, level: u32) -> Option<u64> {
        if let Some(r) = self.table.as_ref().and_then(|t| t.required(level)) {
            return Some(r);
        }
        self.best_ref
            .filter(|b| b.level == level)
            .and_then(|b| b.estimated_required())
    }

    pub fn exp_to_next_level(&self) -> Option<u64> {
        let last = self.last?;
        self.required(last.level).map(|r| r.saturating_sub(last.exp))
    }

    pub fn feed(&mut self, r: ExpReading) -> Ingest {
        match self.evaluate(&r) {
            Ok(None) => {
                self.accept(r);
                Ingest::Baseline
            }
            Ok(Some((exp, levels))) => {
                self.gained += exp;
                self.levels_gained += levels;
                self.accept(r);
                Ingest::Gained { exp, levels }
            }
            Err(reason) => {
                self.rejects_in_row += 1;
                if self.rejects_in_row >= RESYNC_AFTER && reason != Reject::PercentOutOfRange {
                    self.best_ref = None;
                    self.accept(r);
                    Ingest::Resynced
                } else {
                    Ingest::Rejected(reason)
                }
            }
        }
    }

    fn accept(&mut self, r: ExpReading) {
        let keep_ref = self
            .best_ref
            .is_some_and(|b| b.level == r.level && b.percent >= r.percent);
        if !keep_ref {
            self.best_ref = Some(r);
        }
        self.last = Some(r);
        self.rejects_in_row = 0;
    }

    // Returns gained (exp, levels) or None for a baseline reading.
    fn evaluate(&self, r: &ExpReading) -> Result<Option<(u64, u32)>, Reject> {
        if !(0.0..=100.0).contains(&r.percent) {
            return Err(Reject::PercentOutOfRange);
        }
        let Some(last) = self.last else {
            return Ok(None);
        };
        if r.level < last.level {
            return Err(Reject::LevelDecreased);
        }
        if r.level == last.level {
            if r.exp < last.exp {
                return Err(Reject::ExpDecreased);
            }
            self.check_consistency(r)?;
            return Ok(Some((r.exp - last.exp, 0)));
        }
        let mut gain = self
            .required(last.level)
            .ok_or(Reject::UnknownRequired)?
            .saturating_sub(last.exp);
        for lvl in last.level + 1..r.level {
            gain += self
                .table
                .as_ref()
                .and_then(|t| t.required(lvl))
                .ok_or(Reject::UnknownRequired)?;
        }
        gain += r.exp;
        Ok(Some((gain, r.level - last.level)))
    }

    // Catches OCR misreads such as an extra or missing digit.
    fn check_consistency(&self, r: &ExpReading) -> Result<(), Reject> {
        let Some(required) = self.required(r.level).filter(|&q| q > 0) else {
            return Ok(());
        };
        let expected = r.exp as f64 * 100.0 / required as f64;
        // Estimates carry the HUD rounding error of the reference percent.
        let ref_pct = self
            .table
            .as_ref()
            .and_then(|t| t.required(r.level))
            .map(|_| 100.0)
            .or_else(|| self.best_ref.map(|b| b.percent))
            .unwrap_or(100.0)
            .max(0.01);
        let tolerance = 0.05 + r.percent * 0.01 / ref_pct;
        if (expected - r.percent).abs() > tolerance {
            Err(Reject::Inconsistent)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rd(level: u32, exp: u64, percent: f64) -> ExpReading {
        ExpReading::new(level, exp, percent)
    }

    #[test]
    fn accumulates_positive_deltas() {
        let mut t = ExpTracker::new(None);
        assert_eq!(t.feed(rd(30, 1_000, 10.0)), Ingest::Baseline);
        assert_eq!(t.feed(rd(30, 1_500, 15.0)), Ingest::Gained { exp: 500, levels: 0 });
        assert_eq!(t.feed(rd(30, 1_500, 15.0)), Ingest::Gained { exp: 0, levels: 0 });
        assert_eq!(t.gained(), 500);
    }

    #[test]
    fn level_up_with_estimated_requirement() {
        let mut t = ExpTracker::new(None);
        t.feed(rd(30, 9_000, 90.0));
        assert_eq!(t.feed(rd(31, 200, 1.0)), Ingest::Gained { exp: 1_200, levels: 1 });
        assert_eq!(t.levels_gained(), 1);
    }

    #[test]
    fn level_up_with_table_spans_levels() {
        let table = ExpTable::new(vec![10, 20, 30, 40]);
        let mut t = ExpTracker::new(Some(table));
        t.feed(rd(1, 5, 50.0));
        // 5 to finish lvl 1, 20 for lvl 2, then 6 into lvl 3.
        assert_eq!(t.feed(rd(3, 6, 20.0)), Ingest::Gained { exp: 31, levels: 2 });
    }

    #[test]
    fn rejects_decrease_then_resyncs() {
        let mut t = ExpTracker::new(None);
        t.feed(rd(30, 5_000, 50.0));
        assert_eq!(t.feed(rd(30, 4_000, 40.0)), Ingest::Rejected(Reject::ExpDecreased));
        assert_eq!(t.feed(rd(30, 4_000, 40.0)), Ingest::Rejected(Reject::ExpDecreased));
        assert_eq!(t.feed(rd(30, 4_000, 40.0)), Ingest::Resynced);
        assert_eq!(t.gained(), 0);
        assert_eq!(t.feed(rd(30, 4_100, 41.0)), Ingest::Gained { exp: 100, levels: 0 });
    }

    #[test]
    fn rejects_extra_digit_misread() {
        let mut t = ExpTracker::new(None);
        t.feed(rd(50, 123_456, 12.34));
        assert_eq!(
            t.feed(rd(50, 1_234_567, 12.35)),
            Ingest::Rejected(Reject::Inconsistent)
        );
        assert_eq!(t.feed(rd(50, 123_556, 12.35)), Ingest::Gained { exp: 100, levels: 0 });
    }

    #[test]
    fn rejects_out_of_range_percent_without_resync() {
        let mut t = ExpTracker::new(None);
        t.feed(rd(10, 10, 1.0));
        for _ in 0..5 {
            assert_eq!(
                t.feed(rd(10, 20, 250.0)),
                Ingest::Rejected(Reject::PercentOutOfRange)
            );
        }
    }

    #[test]
    fn exp_to_next_level_uses_estimate() {
        let mut t = ExpTracker::new(None);
        t.feed(rd(40, 2_500, 25.0));
        assert_eq!(t.exp_to_next_level(), Some(7_500));
    }
}
