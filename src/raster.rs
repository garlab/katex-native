//! Rasterizes a [`Layout`] with resvg, for renderers that draw bitmaps.

use std::sync::{Arc, OnceLock};

use resvg::{
    tiny_skia,
    usvg::{self, fontdb},
};

use crate::{Color, FONTS, Layout, svg};

/// A rendered formula.
pub struct Raster {
    pub width: u32,
    pub height: u32,
    /// The formula's baseline, in pixels from the top.
    pub baseline: f64,
    /// Straight (not premultiplied) RGBA, row by row.
    pub rgba: Vec<u8>,
}

/// Renders `layout` at `px_per_em` pixels per em, in `color` wherever the
/// formula doesn't set one. `None` if the result would be empty.
///
/// The tag isn't drawn: display math puts it against the right edge of the
/// container, so rasterize `layout.tag` separately and place it there.
pub fn rasterize(layout: &Layout, px_per_em: f64, color: Color) -> Option<Raster> {
    let css = format!(
        "rgba({},{},{},{})",
        color.r,
        color.g,
        color.b,
        f64::from(color.a) / 255.
    );
    let source = if layout.tag.is_some() {
        let body = Layout {
            tag: None,
            ..layout.clone()
        };
        svg::to_svg(&body, px_per_em, &css)
    } else {
        svg::to_svg(layout, px_per_em, &css)
    };
    let tree = usvg::Tree::from_str(&source, svg_options()).ok()?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    Some(Raster {
        width: pixmap.width(),
        height: pixmap.height(),
        baseline: layout.height * px_per_em,
        rgba: pixmap
            .pixels()
            .iter()
            .flat_map(|pixel| {
                let pixel = pixel.demultiply();
                [pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]
            })
            .collect(),
    })
}

/// resvg options with KaTeX's fonts registered under their real styles, for
/// rendering [`svg::to_svg`] output. See [`FontFile`](crate::FontFile) for
/// why the files' own flags can't be used.
pub fn svg_options() -> &'static usvg::Options<'static> {
    static OPTIONS: OnceLock<usvg::Options<'static>> = OnceLock::new();
    OPTIONS.get_or_init(|| {
        let mut db = fontdb::Database::new();
        for file in FONTS {
            db.push_face_info(fontdb::FaceInfo {
                id: fontdb::ID::dummy(),
                source: fontdb::Source::Binary(Arc::new(file.data)),
                index: 0,
                families: vec![(
                    file.font.family.name().to_owned(),
                    fontdb::Language::English_UnitedStates,
                )],
                post_script_name: file.postscript_name(),
                style: if file.font.italic {
                    fontdb::Style::Italic
                } else {
                    fontdb::Style::Normal
                },
                weight: if file.font.bold {
                    fontdb::Weight::BOLD
                } else {
                    fontdb::Weight::NORMAL
                },
                stretch: fontdb::Stretch::Normal,
                monospaced: false,
            });
        }
        usvg::Options {
            fontdb: Arc::new(db),
            ..usvg::Options::default()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_with_the_baseline_inside_the_image() {
        let ctx = katex::KatexContext::default();
        let layout = crate::layout(&ctx, r"\frac{a}{b}", true).unwrap();
        let black = Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        let raster = rasterize(&layout, 20., black).unwrap();
        assert_eq!(
            raster.rgba.len(),
            (raster.width * raster.height * 4) as usize
        );
        assert!(raster.baseline > 0. && raster.baseline < f64::from(raster.height));
        // Something was drawn.
        assert!(raster.rgba.chunks(4).any(|pixel| pixel[3] > 200));
    }
}
