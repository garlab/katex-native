//! Native layout for [katex-rs](https://github.com/katex-rs/katex-rs).
//!
//! katex-rs reproduces KaTeX's HTML: its spans carry the vertical layout,
//! spacing and stretchy shapes, while horizontal flow, fonts and sizes come
//! from KaTeX's CSS in a browser. This crate applies those CSS rules with
//! KaTeX's font metrics and returns a display list (text runs, rectangles,
//! filled paths and lines) that any renderer can draw with KaTeX's fonts.
//!
//! All dimensions are in ems of the formula's font size, with the origin at
//! the left end of the baseline and y pointing down.

mod color;
mod engine;
mod font_data;
mod fonts;
mod path;
#[cfg(feature = "raster")]
pub mod raster;
pub mod svg;

pub use color::Color;
pub use fonts::{FONTS, FontFile};
/// The katex-rs version this crate lays out, for its context and settings.
pub use katex;
pub use path::PathCommand;

/// A laid-out formula.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub width: f64,
    /// Extent above the baseline.
    pub height: f64,
    /// Extent below the baseline.
    pub depth: f64,
    pub items: Vec<Item>,
    /// An equation tag such as `(1)`, which display math puts at the right
    /// edge of the available width.
    pub tag: Option<Box<Layout>>,
}

/// Something to draw.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// Text whose baseline starts at (`x`, `y`), `size` ems tall.
    Text {
        x: f64,
        y: f64,
        text: String,
        font: Font,
        size: f64,
        color: Option<Color>,
    },
    /// A filled rectangle; `y` is its top.
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Option<Color>,
    },
    /// A filled path, clipped to `clip`.
    Path {
        commands: Vec<PathCommand>,
        clip: Rect,
        color: Option<Color>,
    },
    /// A straight stroke.
    Line {
        from: (f64, f64),
        to: (f64, f64),
        thickness: f64,
        color: Option<Color>,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A KaTeX font face. Faces KaTeX doesn't ship (an italic AMS, say) are
/// for the renderer to synthesize or substitute.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Font {
    pub family: Family,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Family {
    #[default]
    Main,
    Math,
    Ams,
    Caligraphic,
    Fraktur,
    SansSerif,
    Script,
    Typewriter,
    Size1,
    Size2,
    Size3,
    Size4,
}

impl Family {
    /// The family name in KaTeX's font files, e.g. `KaTeX_Main`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Main => "KaTeX_Main",
            Self::Math => "KaTeX_Math",
            Self::Ams => "KaTeX_AMS",
            Self::Caligraphic => "KaTeX_Caligraphic",
            Self::Fraktur => "KaTeX_Fraktur",
            Self::SansSerif => "KaTeX_SansSerif",
            Self::Script => "KaTeX_Script",
            Self::Typewriter => "KaTeX_Typewriter",
            Self::Size1 => "KaTeX_Size1",
            Self::Size2 => "KaTeX_Size2",
            Self::Size3 => "KaTeX_Size3",
            Self::Size4 => "KaTeX_Size4",
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// katex-rs rejected the TeX.
    Parse(katex::ParseError),
    /// katex-rs produced a structure this crate doesn't lay out yet.
    Unsupported(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "{err}"),
            Self::Unsupported(what) => write!(f, "unsupported KaTeX output: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<katex::ParseError> for Error {
    fn from(err: katex::ParseError) -> Self {
        Self::Parse(err)
    }
}

/// Lays out `tex` with katex-rs. `ctx` holds katex-rs's function and symbol
/// tables; build it once with `KatexContext::default()` and reuse it.
pub fn layout(ctx: &katex::KatexContext, tex: &str, display: bool) -> Result<Layout, Error> {
    let settings = katex::Settings::builder().display_mode(display).build();
    layout_with(ctx, tex, &settings)
}

/// [`layout`] with any katex-rs settings: macros, `\tag` placement, trust and
/// so on.
pub fn layout_with(
    ctx: &katex::KatexContext,
    tex: &str,
    settings: &katex::Settings,
) -> Result<Layout, Error> {
    let tree = katex::render_to_html_tree(ctx, tex, settings)?;
    engine::layout(ctx, &tree, settings.display_mode)
}
