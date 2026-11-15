use std::fmt::Write as _;

use crate::glyph::Glyph;
use crate::mask::Mask;

/// Max differing pixels still accepted as a match.
const MAX_DISTANCE: usize = 1;

/// A known character shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub ch: char,
    pub top: i32,
    pub mask: Mask,
}

/// Collection of templates for one font.
#[derive(Debug, Clone, Default)]
pub struct GlyphSet {
    templates: Vec<Template>,
}

impl GlyphSet {
    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Known characters, sorted.
    pub fn chars(&self) -> Vec<char> {
        let mut c: Vec<char> = self.templates.iter().map(|t| t.ch).collect();
        c.sort_unstable();
        c.dedup();
        c
    }

    /// Adds a template unless an identical shape is already known; returns true when added.
    pub fn add(&mut self, ch: char, glyph: &Glyph) -> bool {
        if self.lookup(glyph).is_some_and(|(c, d)| c == ch && d == 0) {
            return false;
        }
        self.templates.push(Template { ch, top: glyph.top, mask: glyph.mask.clone() });
        true
    }

    pub fn merge(&mut self, other: &GlyphSet) {
        for t in &other.templates {
            let g = Glyph { x: 0, top: t.top, mask: t.mask.clone() };
            self.add(t.ch, &g);
        }
    }

    /// Best matching character and its pixel distance.
    pub fn lookup(&self, glyph: &Glyph) -> Option<(char, usize)> {
        self.templates
            .iter()
            .filter(|t| (t.top - glyph.top).abs() <= 1)
            .filter_map(|t| t.mask.hamming(&glyph.mask).map(|d| (t.ch, d)))
            .min_by_key(|(_, d)| *d)
            .filter(|(_, d)| *d <= MAX_DISTANCE)
    }

    /// Parses the text format written by [`GlyphSet::to_text`]; `;` starts a comment line.
    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut set = Self::default();
        let mut lines = text.lines().map(str::trim_end).peekable();
        while let Some(line) = lines.next() {
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let (Some("glyph"), Some(ch), Some(top)) = (parts.next(), parts.next(), parts.next()) else {
                return Err(format!("bad header: {line}"));
            };
            let ch = parse_char(ch).ok_or_else(|| format!("bad char: {ch}"))?;
            let top: i32 = top.parse().map_err(|_| format!("bad top: {top}"))?;
            let mut rows = Vec::new();
            while let Some(r) = lines.peek().filter(|r| !r.is_empty() && !r.starts_with("glyph")) {
                rows.push(*r);
                lines.next();
            }
            let mask = Mask::from_rows(&rows).ok_or_else(|| format!("bad rows for {ch:?}"))?;
            set.templates.push(Template { ch, top, mask });
        }
        Ok(set)
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for t in &self.templates {
            let _ = writeln!(out, "glyph {} {}", char_token(t.ch), t.top);
            for row in t.mask.to_rows() {
                let _ = writeln!(out, "{row}");
            }
            out.push('\n');
        }
        out
    }
}

// Space and ';' would break the header, so they get names.
fn char_token(c: char) -> String {
    match c {
        ' ' => "space".into(),
        ';' => "semicolon".into(),
        c => c.to_string(),
    }
}

fn parse_char(s: &str) -> Option<char> {
    match s {
        "space" => Some(' '),
        "semicolon" => Some(';'),
        s => {
            let mut it = s.chars();
            let c = it.next()?;
            it.next().is_none().then_some(c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(rows: &[&str], top: i32) -> Glyph {
        Glyph { x: 0, top, mask: Mask::from_rows(rows).unwrap() }
    }

    #[test]
    fn text_roundtrip() {
        let mut s = GlyphSet::default();
        s.add('1', &g(&[".#", "##", ".#"], 0));
        s.add(',', &g(&["#", "#"], 5));
        let back = GlyphSet::from_text(&s.to_text()).unwrap();
        assert_eq!(back.templates(), s.templates());
    }

    #[test]
    fn lookup_respects_top_and_distance() {
        let mut s = GlyphSet::default();
        s.add('.', &g(&["#"], 6));
        s.add('\'', &g(&["#"], 0));
        assert_eq!(s.lookup(&g(&["#"], 6)), Some(('.', 0)));
        assert_eq!(s.lookup(&g(&["#"], 0)), Some(('\'', 0)));
        assert_eq!(s.lookup(&g(&["#"], 3)), None);
        s.add('7', &g(&["###", "..#", "..#"], 0));
        assert_eq!(s.lookup(&g(&["###", "..#", ".##"], 0)), Some(('7', 1)));
        assert_eq!(s.lookup(&g(&["###", "#.#", ".##"], 0)), None);
    }

    #[test]
    fn add_skips_duplicates() {
        let mut s = GlyphSet::default();
        assert!(s.add('1', &g(&["#"], 0)));
        assert!(!s.add('1', &g(&["#"], 0)));
        assert_eq!(s.len(), 1);
    }
}
