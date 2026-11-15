use crate::mask::Mask;
use crate::parse::parse_exp;
use crate::read::LineRead;
use crate::set::GlyphSet;

/// Relative tolerance when checking exp/percent against the known required EXP.
const REL_TOLERANCE: f64 = 0.0005;

/// Adds templates for unknown glyphs when the true text is known; returns how many were added.
pub fn learn_expected(line: &LineRead, expected: &str, set: &mut GlyphSet) -> usize {
    let chars: Vec<char> = expected.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() != line.glyphs.len() {
        return 0;
    }
    line.unknown
        .iter()
        .filter(|&&i| set.add(chars[i], &line.glyphs[i]))
        .count()
}

/// Guesses unknown digits in an EXP line using the EXP required for the level.
///
/// Every distinct unknown shape is tried as each digit; the guess is returned only when exactly
/// one assignment yields an exp/percent pair consistent with `required`.
pub fn infer_exp_digits(line: &LineRead, required: u64) -> Option<String> {
    if line.unknown.is_empty() || required == 0 {
        return None;
    }
    let mut shapes: Vec<&Mask> = Vec::new();
    let mut shape_of = Vec::with_capacity(line.unknown.len());
    for &i in &line.unknown {
        let m = &line.glyphs[i].mask;
        let idx = shapes.iter().position(|s| *s == m).unwrap_or_else(|| {
            shapes.push(m);
            shapes.len() - 1
        });
        shape_of.push(idx);
    }
    if shapes.len() > 3 {
        return None;
    }
    let mut found: Option<String> = None;
    let combos = 10usize.pow(shapes.len() as u32);
    for n in 0..combos {
        let digits: Vec<char> = (0..shapes.len())
            .map(|k| char::from(b'0' + ((n / 10usize.pow(k as u32)) % 10) as u8))
            .collect();
        let mut text: Vec<char> = line.text.chars().collect();
        for (j, &i) in line.unknown.iter().enumerate() {
            text[i] = digits[shape_of[j]];
        }
        let text: String = text.into_iter().collect();
        if consistent(&text, required) {
            if found.is_some() {
                return None;
            }
            found = Some(text);
        }
    }
    found
}

fn consistent(text: &str, required: u64) -> bool {
    let Some((exp, pct)) = parse_exp(text) else {
        return false;
    };
    let expected = exp as f64 * 100.0 / required as f64;
    // Tight on purpose: an ambiguous guess returns None instead of learning a wrong glyph.
    (expected - pct).abs() <= 0.006 + pct * REL_TOLERANCE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::Glyph;

    fn line(text: &str, unknown: &[usize]) -> LineRead {
        // Each char gets a distinct fake shape; unknown ones share shape by their true char.
        let glyphs = text
            .chars()
            .map(|c| {
                let mut m = Mask::new(8, 1);
                m.set((c as u32) % 8, 0, true);
                m.set(((c as u32) / 8) % 8, 0, true);
                Glyph { x: 0, top: 0, mask: m }
            })
            .collect();
        let mut t: Vec<char> = text.chars().collect();
        for &i in unknown {
            t[i] = '?';
        }
        LineRead { text: t.into_iter().collect(), glyphs, unknown: unknown.to_vec() }
    }

    #[test]
    fn infers_single_unknown_digit() {
        // 54,244 / 174,194 = 31.14%; the '2' is unknown.
        let l = line("54,244[31.14%]", &[3]);
        assert_eq!(infer_exp_digits(&l, 174_194).as_deref(), Some("54,244[31.14%]"));
    }

    #[test]
    fn infers_repeated_unknown_shape() {
        let l = line("54,244[31.14%]", &[1, 4, 5, 11]);
        assert_eq!(infer_exp_digits(&l, 174_194).as_deref(), Some("54,244[31.14%]"));
    }

    #[test]
    fn refuses_without_consistent_answer() {
        let l = line("54,244[31.14%]", &[3]);
        assert_eq!(infer_exp_digits(&l, 999_999), None);
    }

    #[test]
    fn learns_when_counts_match() {
        let l = line("36", &[1]);
        let mut set = GlyphSet::default();
        assert_eq!(learn_expected(&l, "36", &mut set), 1);
        assert_eq!(set.chars(), vec!['6']);
        assert_eq!(learn_expected(&l, "367", &mut set), 0);
    }
}
