//! Writes a [`Layout`] as standalone SVG. Text refers to KaTeX's fonts by
//! family name, so the reader needs them installed or loaded.

use std::fmt::Write as _;

use crate::{Color, Item, Layout, PathCommand};

/// Renders `layout` at `px_per_em` pixels per em, with `color` (any SVG
/// color) wherever the formula doesn't set one. A tag follows the formula
/// after a 1 em gap.
pub fn to_svg(layout: &Layout, px_per_em: f64, color: &str) -> String {
    let gap = 1.;
    let tag_width = layout.tag.as_ref().map_or(0., |tag| gap + tag.width);
    let height = layout
        .height
        .max(layout.tag.as_ref().map_or(0., |tag| tag.height));
    let depth = layout
        .depth
        .max(layout.tag.as_ref().map_or(0., |tag| tag.depth));
    let mut svg = String::new();
    write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 {y} {vw} {vh}">"#,
        w = (layout.width + tag_width) * px_per_em,
        h = (height + depth) * px_per_em,
        y = -height,
        vw = layout.width + tag_width,
        vh = height + depth,
    )
    .ok();
    let mut clips = 0;
    write_items(&mut svg, &layout.items, 0., color, &mut clips);
    if let Some(tag) = &layout.tag {
        write_items(&mut svg, &tag.items, layout.width + gap, color, &mut clips);
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
