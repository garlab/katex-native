//! Renders TeX to a PNG through the crate's SVG output, with KaTeX's fonts:
//! `cargo run --example render -- '\frac{a}{b}' out.png [display] [px-per-em]`.

use resvg::{tiny_skia, usvg};

fn main() {
    let mut args = std::env::args().skip(1);
    let tex = args.next().expect("TeX expression");
    let output = args.next().expect("output PNG path");
    let display = args.next().is_some_and(|mode| mode == "display");
    let px_per_em: f64 = args
        .next()
        .map_or(40., |px| px.parse().expect("pixels per em"));

    let ctx = katex::KatexContext::default();
    let layout = katex_native::layout(&ctx, &tex, display).expect("layout");
    let svg = katex_native::svg::to_svg(&layout, px_per_em, "black");

    let options = katex_native::raster::svg_options();
    let tree = usvg::Tree::from_str(&svg, options).expect("valid SVG");
    let size = tree.size().to_int_size();
    let margin = 8;
    let mut pixmap = tiny_skia::Pixmap::new(size.width() + 2 * margin, size.height() + 2 * margin)
        .expect("size");
    pixmap.fill(tiny_skia::Color::WHITE);
    let offset = tiny_skia::Transform::from_translate(margin as f32, margin as f32);
    resvg::render(&tree, offset, &mut pixmap.as_mut());
    pixmap.save_png(&output).expect("write PNG");
    println!(
        "{output}: {:.3} x ({:.3} + {:.3}) em, {} items",
        layout.width,
        layout.height,
        layout.depth,
        layout.items.len()
    );
}
