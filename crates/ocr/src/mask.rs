use image::RgbaImage;

/// Rectangle in image pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Intersection with an image of the given size.
    pub fn clamp(self, width: u32, height: u32) -> Self {
        let x = self.x.min(width);
        let y = self.y.min(height);
        Self { x, y, w: self.w.min(width - x), h: self.h.min(height - y) }
    }
}

/// Which pixels count as ink for a given font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    /// Status bar text: white glyphs plus green brackets.
    Status,
    /// Pure white pixels only (labels, level digits on orange tiles).
    White,
    /// Orange level tile background.
    Orange,
}

impl Ink {
    pub fn test(self, [r, g, b, _]: [u8; 4]) -> bool {
        let white = r.min(g).min(b) > 200;
        match self {
            Ink::Status => white || (g > 150 && r < 180 && b < 100),
            Ink::White => white,
            Ink::Orange => r > 200 && g < 170 && b < 80,
        }
    }
}

/// Binary bitmap, row-major.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mask {
    pub w: u32,
    pub h: u32,
    pub bits: Vec<bool>,
}

impl Mask {
    pub fn new(w: u32, h: u32) -> Self {
        Self { w, h, bits: vec![false; (w * h) as usize] }
    }

    pub fn from_image(img: &RgbaImage, rect: Rect, ink: Ink) -> Self {
        let r = rect.clamp(img.width(), img.height());
        let mut m = Self::new(r.w, r.h);
        for y in 0..r.h {
            for x in 0..r.w {
                if ink.test(img.get_pixel(r.x + x, r.y + y).0) {
                    m.set(x, y, true);
                }
            }
        }
        m
    }

    pub fn get(&self, x: u32, y: u32) -> bool {
        self.bits[(y * self.w + x) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, v: bool) {
        self.bits[(y * self.w + x) as usize] = v;
    }

    pub fn col_has_ink(&self, x: u32) -> bool {
        (0..self.h).any(|y| self.get(x, y))
    }

    pub fn row_has_ink(&self, y: u32) -> bool {
        (0..self.w).any(|x| self.get(x, y))
    }

    pub fn ink_count(&self) -> usize {
        self.bits.iter().filter(|b| **b).count()
    }

    /// Copy of the sub-rectangle.
    pub fn sub(&self, r: Rect) -> Self {
        let mut m = Self::new(r.w, r.h);
        for y in 0..r.h {
            for x in 0..r.w {
                m.set(x, y, self.get(r.x + x, r.y + y));
            }
        }
        m
    }

    /// Rows as `#`/`.` strings.
    pub fn to_rows(&self) -> Vec<String> {
        (0..self.h)
            .map(|y| (0..self.w).map(|x| if self.get(x, y) { '#' } else { '.' }).collect())
            .collect()
    }

    pub fn from_rows(rows: &[&str]) -> Option<Self> {
        let h = rows.len() as u32;
        let w = rows.first()?.chars().count() as u32;
        let mut m = Self::new(w, h);
        for (y, row) in rows.iter().enumerate() {
            if row.chars().count() as u32 != w {
                return None;
            }
            for (x, c) in row.chars().enumerate() {
                m.set(x as u32, y as u32, c == '#');
            }
        }
        Some(m)
    }

    pub fn hamming(&self, other: &Self) -> Option<usize> {
        (self.w == other.w && self.h == other.h)
            .then(|| self.bits.iter().zip(&other.bits).filter(|(a, b)| a != b).count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_roundtrip() {
        let m = Mask::from_rows(&["#.", ".#"]).unwrap();
        assert_eq!(m.to_rows(), vec!["#.", ".#"]);
        assert_eq!(m.ink_count(), 2);
    }

    #[test]
    fn rect_clamps() {
        assert_eq!(Rect::new(8, 8, 5, 5).clamp(10, 10), Rect::new(8, 8, 2, 2));
    }
}
