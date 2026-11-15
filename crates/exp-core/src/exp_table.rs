/// EXP to next level for MapleStory Classic, levels 1..=99 (cap 100), from public game data.
const CLASSIC: [u64; 99] = [
    15, 34, 57, 92, 135, 372, 560, 840, 1242, 1716, // 1-10
    2360, 3216, 4200, 5460, 7050, 8840, 11040, 13716, 16680, 20216, // 11-20
    24402, 28980, 34320, 40512, 47216, 54900, 63666, 73080, 83720, 95700, // 21-30
    108480, 122760, 138666, 155540, 174216, 194832, 216600, 240500, 266682, 294216, // 31-40
    324240, 356916, 391160, 428280, 468450, 510420, 555680, 604416, 655200, 709716, // 41-50
    748608, 789631, 832902, 878545, 926689, 977471, 1031036, 1087536, 1147132, 1209994, // 51-60
    1276301, 1346242, 1420016, 1497832, 1579913, 1666492, 1757815, 1854143, 1955750, 2062925, // 61-70
    2175973, 2295216, 2420993, 2553663, 2693603, 2841212, 2996910, 3161140, 3334370, 3517093, // 71-80
    3709829, 3913127, 4127566, 4353756, 4592341, 4844001, 5109452, 5389449, 5684790, 11938059, // 81-90
    25069923, 52646838, 110558359, 232172553, 487562361, 1023880958, 2150150011, 4515315023, 9482161548, // 91-99
];

/// EXP required to go from each level to the next.
#[derive(Debug, Clone, Default)]
pub struct ExpTable {
    // Index 0 is level 1.
    required: Vec<u64>,
}

impl ExpTable {
    /// Builds a table where `required[i]` is the EXP needed at level `i + 1`.
    pub fn new(required: Vec<u64>) -> Self {
        Self { required }
    }

    /// The MapleStory Classic table.
    pub fn classic() -> Self {
        Self::new(CLASSIC.to_vec())
    }

    pub fn required(&self, level: u32) -> Option<u64> {
        level
            .checked_sub(1)
            .and_then(|i| self.required.get(i as usize).copied())
    }

    /// Level whose requirement matches the HUD's exp and percent, if exactly one fits.
    pub fn level_for(&self, exp: u64, percent: f64) -> Option<u32> {
        if percent <= 0.0 {
            return None;
        }
        // The HUD shows 2 decimals; allow a rounding or truncation error of 0.01 points.
        let tolerance = 0.0101 / percent + 0.0005;
        let estimate = exp as f64 * 100.0 / percent;
        let mut fits = self.required.iter().enumerate().filter(|&(_, &req)| {
            let req = req as f64;
            (estimate - req).abs() / req <= tolerance && exp < req as u64
        });
        let (i, _) = fits.next()?;
        fits.next().is_none().then_some(i as u32 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_one_based() {
        let t = ExpTable::new(vec![15, 34, 57]);
        assert_eq!(t.required(0), None);
        assert_eq!(t.required(1), Some(15));
        assert_eq!(t.required(3), Some(57));
        assert_eq!(t.required(4), None);
    }

    #[test]
    fn classic_covers_levels_1_to_99() {
        let t = ExpTable::classic();
        assert_eq!(t.required(1), Some(15));
        assert_eq!(t.required(35), Some(174_216));
        assert_eq!(t.required(99), Some(9_482_161_548));
        assert_eq!(t.required(100), None);
    }

    #[test]
    fn level_from_hud_values() {
        let t = ExpTable::classic();
        // Real capture: LV 35, EXP 54,244 [31.14%].
        assert_eq!(t.level_for(54_244, 31.14), Some(35));
        assert_eq!(t.level_for(1_000_000, 50.0), None);
        assert_eq!(t.level_for(10, 0.0), None);
    }
}
