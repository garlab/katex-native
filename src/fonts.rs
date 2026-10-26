use crate::{Family, Font};

/// One of KaTeX's font files. They are licensed under the SIL Open Font
/// License 1.1 (`fonts/OFL.txt`), which applies when you redistribute them.
///
/// Register each file under its `font`'s family, weight and style. The files
/// themselves all claim to be upright and regular weight (bold ones do set
/// weight 700), so a font system that reads their flags can't tell
/// `KaTeX_Main` Regular from Italic. KaTeX's CSS sets the style in
/// `@font-face` instead.
#[derive(Clone, Copy, Debug)]
pub struct FontFile {
    pub font: Font,
    pub data: &'static [u8],
}

impl FontFile {
    /// The PostScript name inside the file, e.g. `KaTeX_Main-BoldItalic`.
    pub fn postscript_name(&self) -> String {
        let style = match (self.font.bold, self.font.italic) {
            (false, false) => "Regular",
            (true, false) => "Bold",
            (false, true) => "Italic",
            (true, true) => "BoldItalic",
        };
        format!("{}-{style}", self.font.family.name())
    }
}

macro_rules! font {
    ($family:ident, $bold:literal, $italic:literal, $file:literal) => {
        FontFile {
            font: Font {
                family: Family::$family,
                bold: $bold,
                italic: $italic,
            },
            data: include_bytes!(concat!("../fonts/KaTeX_", $file, ".ttf")),
        }
    };
}

/// Every face KaTeX ships, about 0.5 MB in all.
pub static FONTS: &[FontFile] = &[
    font!(Ams, false, false, "AMS-Regular"),
    font!(Caligraphic, true, false, "Caligraphic-Bold"),
    font!(Caligraphic, false, false, "Caligraphic-Regular"),
    font!(Fraktur, true, false, "Fraktur-Bold"),
    font!(Fraktur, false, false, "Fraktur-Regular"),
    font!(Main, true, false, "Main-Bold"),
    font!(Main, true, true, "Main-BoldItalic"),
    font!(Main, false, true, "Main-Italic"),
    font!(Main, false, false, "Main-Regular"),
    font!(Math, true, true, "Math-BoldItalic"),
    font!(Math, false, true, "Math-Italic"),
    font!(SansSerif, true, false, "SansSerif-Bold"),
    font!(SansSerif, false, true, "SansSerif-Italic"),
    font!(SansSerif, false, false, "SansSerif-Regular"),
    font!(Script, false, false, "Script-Regular"),
    font!(Size1, false, false, "Size1-Regular"),
    font!(Size2, false, false, "Size2-Regular"),
    font!(Size3, false, false, "Size3-Regular"),
    font!(Size4, false, false, "Size4-Regular"),
    font!(Typewriter, false, false, "Typewriter-Regular"),
];
