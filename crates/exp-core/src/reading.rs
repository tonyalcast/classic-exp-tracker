use serde::{Deserialize, Serialize};

/// One validated-shape sample of the EXP HUD.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExpReading {
    pub level: u32,
    pub exp: u64,
    /// Percentage as shown on the HUD, 0.0..=100.0.
    pub percent: f64,
}

impl ExpReading {
    pub fn new(level: u32, exp: u64, percent: f64) -> Self {
        Self { level, exp, percent }
    }

    /// Estimated EXP required for the current level, derived from exp and percent.
    pub fn estimated_required(&self) -> Option<u64> {
        (self.percent > 0.0).then(|| (self.exp as f64 * 100.0 / self.percent).round() as u64)
    }
}
