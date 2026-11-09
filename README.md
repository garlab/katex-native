# katex-native

[crates.io](https://crates.io/crates/katex-native) · [API documentation](https://docs.rs/katex-native)

Native layout for [katex-rs](https://github.com/katex-rs/katex-rs), the Rust port of KaTeX.

katex-rs produces KaTeX's HTML. Its spans already carry the vertical layout, spacing and stretchy shapes, but a browser supplies the rest from KaTeX's CSS: horizontal flow, font choice and sizes. This crate applies those CSS rules with KaTeX's fonts and returns a display list that any renderer can draw. The list contains text runs, rectangles, filled paths and lines.

```rust
use katex_native::katex::KatexContext;

let ctx = KatexContext::default();
let layout = katex_native::layout(&ctx, r"\int_0^1 x^2\,dx = \frac{1}{3}", true)?;
for item in &layout.items {
    // Item::Text { x, y, text, font, size, color }, Item::Rect, Item::Path, Item::Line
}
```

Dimensions are in ems of the formula's font size, with the origin at the left end of the baseline and y pointing down. A tag or equation numbers come separately in `Layout::tag`, for the renderer to put against the right edge of the container. `layout_numbered` continues automatic equation numbers from an earlier formula, and `Layout::equation_numbers` says how many a formula used. Each text run names a KaTeX font face. `FONTS` embeds the 20 KaTeX font files (about 0.5 MB). `svg::to_svg` writes a layout as SVG. With the `raster` feature, `raster::rasterize` renders it to RGBA through resvg.

## Installation

```sh
cargo add katex-native                    # layout and SVG
cargo add katex-native --features raster  # also RGBA rasters through resvg
```

Rust 1.88 or newer. katex-native 0.1 uses katex-rs 0.3 and re-exports it as `katex_native::katex`, so the context and settings types you pass in always match. Moving to a katex-rs release that changes those types is a breaking release of this crate.

## Registering the fonts

Every KaTeX font file declares itself upright and regular weight; bold files at least set weight 700. A font system that reads those flags cannot tell `KaTeX_Main` Regular from `KaTeX_Main` Italic. Browsers are unaffected because KaTeX's CSS declares each face's style in `@font-face`. Register each `FontFile` under its `font` field's family, weight and style, as `raster::svg_options` does for resvg.

## Fidelity

Measured against KaTeX's own screenshot tests: the 128 cases from KaTeX's `test/screenshotter/ss_data.yaml` at commit `8000fbd4`, rendered at the screenshotter's 77.44 px per em. For each case, the score is the share of ink within 2 px of the other image's ink, taking the worse direction:

| Score | Cases (of 120 scored) |
|---|---:|
| ≥ 0.99 | 109 |
| ≥ 0.95 | 112 |
| ≥ 0.90 | 114 |

The eight unscored cases are the three katex-rs 0.3 can't parse (`\mapsfrom`, `\reflectbox`, and a deliberately invalid case), and five whose screenshots include text or styles around the math.

Display cases are laid out with `svg::to_svg_display` at the screenshotter's 1008 px page width, which puts tags and equation numbers at the right edge as KaTeX does. `Align` scores 0.92 because KaTeX 0.18 drops an empty last row that the older screenshot still numbers.

Known gaps:
- Commutative diagrams (`CD`).
- Automatic line breaks, which depend on the container's width.
- `\\` outside environments: line spacing is close but not exact.
- `\includegraphics`, which is laid out but not drawn.
- Characters missing from KaTeX's fonts, for example CJK: their width is estimated.

Layout, including katex-rs's parsing, takes about 0.1 ms per formula at the median and under 1 ms at worst (release build, Apple M2 Pro).

[CONTRIBUTING.md](CONTRIBUTING.md) explains how to reproduce these scores.

## License

MIT. `src/engine.rs` translates KaTeX's stylesheet, which is MIT-licensed by Khan Academy and other contributors; `LICENSE` includes its notice. KaTeX's fonts in `fonts/` are under the SIL Open Font License 1.1 (`fonts/OFL.txt`), with Reserved Font Names `KaTeX_*`. The library embeds them, so programs that use it distribute them.

To work on the crate, see [CONTRIBUTING.md](CONTRIBUTING.md).
