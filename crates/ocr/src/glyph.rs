use crate::mask::{Mask, Rect};

/// One segmented character: its trimmed bitmap and placement in the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph {
    /// Left edge within the source mask.
    pub x: u32,
    /// Top edge relative to the line top (tallest glyphs start at 0).
    pub top: i32,
    pub mask: Mask,
}

/// Splits a line mask into glyphs separated by empty columns.
pub fn segment(line: &Mask) -> Vec<Glyph> {
    let mut raw = Vec::new();
    let mut x = 0;
    while x < line.w {
        if !line.col_has_ink(x) {
            x += 1;
            continue;
        }
        let start = x;
        while x < line.w && line.col_has_ink(x) {
            x += 1;
        }
        let cols = line.sub(Rect::new(start, 0, x - start, line.h));
        let top = (0..cols.h).find(|&y| cols.row_has_ink(y)).unwrap_or(0);
        let bottom = (0..cols.h).rev().find(|&y| cols.row_has_ink(y)).unwrap_or(0);
        let mask = cols.sub(Rect::new(0, top, cols.w, bottom - top + 1));
        raw.push((start, top, mask));
    }
    // Normalize vertical placement against the tallest glyph so ROI shifts do not matter.
    let line_top = raw
        .iter()
        .max_by_key(|(_, t, m)| (m.h, std::cmp::Reverse(*t)))
        .map_or(0, |(_, t, _)| *t);
    raw.into_iter()
        .map(|(x, top, mask)| Glyph { x, top: top as i32 - line_top as i32, mask })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_blank_columns_and_normalizes_top() {
        let line = Mask::from_rows(&[
            "......",
            "#..#..",
            "#..#..",
            "#.....",
            "#..#.#",
        ])
        .unwrap();
        let g = segment(&line);
        assert_eq!(g.len(), 3);
        assert_eq!((g[0].x, g[0].top, g[0].mask.h), (0, 0, 4));
        assert_eq!((g[1].x, g[1].top), (3, 0));
        assert_eq!(g[1].mask.to_rows(), vec!["#", "#", ".", "#"]);
        assert_eq!((g[2].x, g[2].top, g[2].mask.h), (5, 3, 1));
    }
}
