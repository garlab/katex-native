//! Renders KaTeX's screenshot test cases to PNGs for comparison with KaTeX's
//! reference images:
//! `cargo run --release --example corpus -- cases.json out-dir [px-per-em]`.
//!
//! `cases.json` is a list of `{name, tex, display, options}`; `options.macros`
//! are prepended as `\newcommand`s. KaTeX's screenshotter sets math at 4em on
//! a 16px page, and `.katex` adds 1.21em: 77.44 px per em.

use std::{fs, path::Path, time::Instant};

use resvg::{tiny_skia, usvg};
use serde_json::{Value, json};

fn main() {
    let mut args = std::env::args().skip(1);
    let cases_path = args.next().expect("cases.json");
    let out = args.next().expect("output directory");
    let px_per_em: f64 = args
        .next()
        .map_or(77.44, |px| px.parse().expect("pixels per em"));
    fs::create_dir_all(&out).expect("output directory");
    let cases: Vec<Value> =
        serde_json::from_str(&fs::read_to_string(cases_path).expect("read cases")).expect("JSON");

    let ctx = katex::KatexContext::default();
    let options = katex_native::raster::svg_options();
    let mut report = Vec::new();
    for case in &cases {
        let name = case["name"].as_str().expect("name");
        let mut tex = case["tex"].as_str().expect("tex").to_owned();
        if let Some(macros) = case["options"]["macros"].as_object() {
            for (macro_name, value) in macros {
                tex = format!(
                    "\\newcommand{{{macro_name}}}{{{}}}{tex}",
                    value.as_str().unwrap()
                );
            }
        }
        let display = case["display"].as_bool().unwrap_or(false);
        let started = Instant::now();
        let result = katex_native::layout(&ctx, &tex, display);
        let layout_ms = started.elapsed().as_secs_f64() * 1000.;
        let layout = match result {
            Ok(layout) => layout,
            Err(err) => {
                report.push(json!({ "name": name, "error": err.to_string() }));
                continue;
            }
        };
        // Display math fills the screenshotter's 1008 px wide page.
        let svg = if display {
            katex_native::svg::to_svg_display(&layout, px_per_em, "black", 1008. / px_per_em)
        } else {
            katex_native::svg::to_svg(&layout, px_per_em, "black")
        };
        let tree = usvg::Tree::from_str(&svg, options).expect("valid SVG");
        let size = tree.size().to_int_size();
        let mut pixmap =
            tiny_skia::Pixmap::new(size.width().max(1), size.height().max(1)).expect("size");
        pixmap.fill(tiny_skia::Color::WHITE);
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
        let stem: String = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        pixmap
            .save_png(Path::new(&out).join(format!("{stem}.png")))
            .expect("write PNG");
        report.push(json!({
            "name": name,
            "layout_ms": layout_ms,
            "items": layout.items.len(),
            "width_em": layout.width,
        }));
    }
    fs::write(
        Path::new(&out).join("report.json"),
        serde_json::to_string_pretty(&report).expect("JSON"),
    )
    .expect("write report");
    let failed = report.iter().filter(|r| r.get("error").is_some()).count();
    println!(
        "{} cases, {failed} failed to lay out; PNGs in {out}",
        report.len()
    );
}
