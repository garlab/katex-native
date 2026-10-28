//! Writes a [`Layout`] as standalone SVG. Text refers to KaTeX's fonts by
//! family name, so the reader needs them installed or loaded.

use std::fmt::Write as _;

use crate::{Color, Item, Layout, PathCommand};

/// Renders `layout` at `px_per_em` pixels per em, with `color` (any SVG
/// color) wherever the formula doesn't set one. A tag follows the formula
/// after a 1 em gap.
pub fn to_svg(layout: &Layout, px_per_em: f64, color: &str) -> String {
    let gap = 1.;
    let width = layout.width + layout.tag.as_ref().map_or(0., |tag| gap + tag.width);
    write_svg(layout, px_per_em, color, width, 0., layout.width + gap)
}

/// [`to_svg`] laid out as display math in a block `width` ems wide: the
/// formula centered and its tag against the right edge, as KaTeX's CSS does.
/// The block widens to fit both if needed.
pub fn to_svg_display(layout: &Layout, px_per_em: f64, color: &str, width: f64) -> String {
    let tag_width = layout.tag.as_ref().map_or(0., |tag| tag.width);
    let width = width.max(layout.width + tag_width);
    let body = (width - layout.width) / 2.;
    write_svg(layout, px_per_em, color, width, body, width - tag_width)
}

/// Writes `layout` with the formula at `body` and its tag at `tag` ems from
/// the left, in an SVG `width` ems wide.
fn write_svg(
    layout: &Layout,
    px_per_em: f64,
    color: &str,
    width: f64,
    body: f64,
    tag: f64,
) -> String {
    let height = layout
        .height
        .max(layout.tag.as_ref().map_or(0., |tag| tag.height));
    let depth = layout
        .depth
        .max(layout.tag.as_ref().map_or(0., |tag| tag.depth));
    let mut svg = String::new();
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 {y} {width} {vh}">"#,
        w = width * px_per_em,
        h = (height + depth) * px_per_em,
        y = -height,
        vh = height + depth,
    )
    .ok();
    let mut clips = 0;
    write_items(&mut svg, &layout.items, body, color, &mut clips);
    if let Some(tag_layout) = &layout.tag {
        write_items(&mut svg, &tag_layout.items, tag, color, &mut clips);
    }
    svg.push_str("</svg>");
    svg
}

fn write_items(svg: &mut String, items: &[Item], dx: f64, default: &str, clips: &mut usize) {
    let fill = |color: &Option<Color>| match color {
        Some(c) => format!("rgba({},{},{},{})", c.r, c.g, c.b, f64::from(c.a) / 255.),
        None => default.to_owned(),
    };
    for item in items {
        match item {
            Item::Text {
                x,
                y,
                text,
                font,
                size,
                color,
            } => {
                write!(
                    svg,
                    r#"<text x="{}" y="{y}" font-family="{}" font-size="{size}" font-weight="{}" font-style="{}" fill="{}" xml:space="preserve">"#,
                    x + dx,
                    font.family.name(),
                    if font.bold { "bold" } else { "normal" },
                    if font.italic { "italic" } else { "normal" },
                    fill(color),
                )
                .ok();
                for ch in text.chars() {
                    match ch {
                        '<' => svg.push_str("&lt;"),
                        '>' => svg.push_str("&gt;"),
                        '&' => svg.push_str("&amp;"),
                        _ => svg.push(ch),
                    }
                }
                svg.push_str("</text>");
            }
            Item::Rect {
                x,
                y,
                width,
                height,
                color,
            } => {
                write!(
                    svg,
                    r#"<rect x="{}" y="{y}" width="{width}" height="{height}" fill="{}"/>"#,
                    x + dx,
                    fill(color),
                )
                .ok();
            }
            Item::Path {
                commands,
                clip,
                color,
            } => {
                *clips += 1;
                write!(
                    svg,
                    r#"<clipPath id="c{clips}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath><path clip-path="url(#c{clips})" fill="{}" d=""#,
                    clip.x + dx,
                    clip.y,
                    clip.width,
                    clip.height,
                    fill(color),
                )
                .ok();
                for command in commands {
                    match *command {
                        PathCommand::MoveTo(x, y) => write!(svg, "M{} {y}", x + dx),
                        PathCommand::LineTo(x, y) => write!(svg, "L{} {y}", x + dx),
                        PathCommand::CubicTo(a, b, c, d, e, f) => {
                            write!(svg, "C{} {b} {} {d} {} {f}", a + dx, c + dx, e + dx)
                        }
                        PathCommand::Close => write!(svg, "Z"),
                    }
                    .ok();
                }
                svg.push_str(r#""/>"#);
            }
            Item::Line {
                from,
                to,
                thickness,
                color,
            } => {
                write!(
                    svg,
                    r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke-width="{thickness}" stroke="{}"/>"#,
                    from.0 + dx,
                    from.1,
                    to.0 + dx,
                    to.1,
                    fill(color),
                )
                .ok();
            }
        }
    }
}
