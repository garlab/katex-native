//! Advance widths from KaTeX's TrueType files: the `cmap` and `hmtx` tables
//! are all horizontal layout needs, since the fonts have no kerning.

use std::{collections::HashMap, sync::OnceLock};

use crate::{FONTS, Font};

/// The advance of `ch` in ems, from the face `font` selects. Faces KaTeX
/// doesn't ship fall back to the family's regular face, which is what a
/// browser synthesizes the style from. `None` if the face lacks `ch`.
pub(crate) fn advance(font: Font, ch: char) -> Option<f64> {
    static FACES: OnceLock<HashMap<Font, Face>> = OnceLock::new();
    let faces = FACES.get_or_init(|| {
        FONTS
            .iter()
            .filter_map(|file| Some((file.font, Face::parse(file.data)?)))
            .collect()
    });
    let regular = Font {
        family: font.family,
        bold: false,
        italic: false,
    };
    let bold = Font {
        italic: false,
        ..font
    };
    let face = faces
        .get(&font)
        .or_else(|| faces.get(&bold))
        .or_else(|| faces.get(&regular))?;
    face.advance(ch)
}

/// Line metrics of `KaTeX_Main` (`hhea` ascent and descent), in ems.
pub(crate) const MAIN_ASCENT: f64 = 0.903;
pub(crate) const MAIN_DESCENT: f64 = 0.272;

struct Face {
    data: &'static [u8],
    units_per_em: f64,
    cmap: Cmap,
    hmtx: usize,
    long_metrics: u16,
}

#[derive(Clone, Copy)]
enum Cmap {
    /// Format 4 subtable offset.
    Segments(usize),
    /// Format 12 subtable offset.
    Groups(usize),
}

impl Face {
    fn parse(data: &'static [u8]) -> Option<Self> {
        let tables = read_u16(data, 4)? as usize;
        let table = |tag: &[u8; 4]| -> Option<usize> {
            (0..tables)
                .find_map(|i| {
                    let record = 12 + 16 * i;
                    (data.get(record..record + 4)? == tag).then(|| read_u32(data, record + 8))?
                })
                .map(|offset| offset as usize)
        };
        let head = table(b"head")?;
        let hhea = table(b"hhea")?;
        let cmap = table(b"cmap")?;
        let subtables = read_u16(data, cmap + 2)? as usize;
        let mut best = None;
        for i in 0..subtables {
            let record = cmap + 4 + 8 * i;
            let platform = read_u16(data, record)?;
            let encoding = read_u16(data, record + 2)?;
            let offset = cmap + read_u32(data, record + 4)? as usize;
            match (platform, encoding, read_u16(data, offset)?) {
                // Full Unicode beats the Basic Multilingual Plane.
                (0 | 3, _, 12) => best = Some(Cmap::Groups(offset)),
                (0, _, 4) | (3, 1, 4) if best.is_none() => best = Some(Cmap::Segments(offset)),
                _ => {}
            }
        }
        Some(Self {
            data,
            units_per_em: f64::from(read_u16(data, head + 18)?),
            cmap: best?,
            hmtx: table(b"hmtx")?,
            long_metrics: read_u16(data, hhea + 34)?,
        })
    }

    fn advance(&self, ch: char) -> Option<f64> {
        let glyph = self.glyph(u32::from(ch)).filter(|&glyph| glyph != 0)?;
        // Glyphs past the long metrics share the last advance.
        let index = glyph.min(self.long_metrics.checked_sub(1)?);
        let units = read_u16(self.data, self.hmtx + 4 * usize::from(index))?;
        Some(f64::from(units) / self.units_per_em)
    }

    fn glyph(&self, code: u32) -> Option<u16> {
        let data = self.data;
        match self.cmap {
            Cmap::Segments(table) => {
                let code = u16::try_from(code).ok()?;
                let segments = read_u16(data, table + 6)? as usize / 2;
                let ends = table + 14;
                let starts = ends + 2 * segments + 2;
                let deltas = starts + 2 * segments;
                let ranges = deltas + 2 * segments;
                for i in 0..segments {
                    if code > read_u16(data, ends + 2 * i)? {
                        continue;
                    }
                    let start = read_u16(data, starts + 2 * i)?;
                    if code < start {
                        return None;
                    }
                    let delta = read_u16(data, deltas + 2 * i)?;
                    let range = read_u16(data, ranges + 2 * i)?;
                    if range == 0 {
                        return Some(code.wrapping_add(delta));
                    }
                    let at = ranges + 2 * i + range as usize + 2 * (code - start) as usize;
                    let glyph = read_u16(data, at)?;
                    return (glyph != 0).then(|| glyph.wrapping_add(delta));
                }
                None
            }
            Cmap::Groups(table) => {
                let groups = read_u32(data, table + 12)? as usize;
                (0..groups).find_map(|i| {
                    let group = table + 16 + 12 * i;
                    let start = read_u32(data, group)?;
                    let end = read_u32(data, group + 4)?;
                    (start..=end)
                        .contains(&code)
                        .then(|| read_u32(data, group + 8))?
                        .and_then(|first| u16::try_from(first + code - start).ok())
                })
            }
        }
    }
}

fn read_u16(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn read_u32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Family;

    fn font(family: Family, bold: bool, italic: bool) -> Font {
        Font {
            family,
            bold,
            italic,
        }
    }

    #[test]
    fn reads_advances_from_the_font_files() {
        // KaTeX's own metrics: `x` is 0.52778 ems in Main-Regular, 0.57153
        // in Math-Italic.
        assert_eq!(advance(font(Family::Main, false, false), 'x'), Some(0.528));
        assert_eq!(advance(font(Family::Math, false, true), 'x'), Some(0.572));
        // A face KaTeX doesn't ship falls back to the regular one.
        assert_eq!(
            advance(font(Family::Ams, false, true), 'R'),
            advance(font(Family::Ams, false, false), 'R')
        );
        assert!(advance(font(Family::Size2, false, false), '\u{222c}').unwrap() > 1.);
        assert_eq!(advance(font(Family::Main, false, false), '\u{4e2d}'), None);
    }
}
