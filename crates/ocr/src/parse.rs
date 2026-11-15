/// Parses the EXP line, e.g. `54,244[31.14%]`; anything after `]` is ignored.
pub fn parse_exp(text: &str) -> Option<(u64, f64)> {
    let line = &text[..text.find(']')?];
    let (exp, pct) = line.split_once('[')?;
    let digits: String = exp.chars().filter(|c| *c != ',').collect();
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let pct = pct.strip_suffix('%')?;
    let (int, frac) = pct.split_once('.')?;
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(int) || !all_digits(frac) {
        return None;
    }
    Some((digits.parse().ok()?, pct.parse().ok()?))
}

/// Parses a level made only of digits.
pub fn parse_level(text: &str) -> Option<u32> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_line() {
        assert_eq!(parse_exp("54,244[31.14%]"), Some((54_244, 31.14)));
        assert_eq!(parse_exp("1,234,567[0.05%]??"), Some((1_234_567, 0.05)));
        assert_eq!(parse_exp("0[0.00%]"), Some((0, 0.0)));
        assert_eq!(parse_exp("54,2?4[31.14%]"), None);
        assert_eq!(parse_exp("54,244[31.14]"), None);
        assert_eq!(parse_exp("54,244 31.14%"), None);
    }

    #[test]
    fn level() {
        assert_eq!(parse_level("35"), Some(35));
        assert_eq!(parse_level("3?"), None);
        assert_eq!(parse_level(""), None);
    }
}
