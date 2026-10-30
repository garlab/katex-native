//! Lays out KaTeX's 128 screenshot test cases (`tests/katex/cases.json`,
//! KaTeX's `test/screenshotter/ss_data.yaml` at commit `8000fbd4` as JSON)
//! and compares them with `tests/katex/layouts.txt`. After a change meant to
//! move things, regenerate that file with
//! `UPDATE_LAYOUTS=1 cargo test --test katex_corpus` and review its diff.

use std::{
    any::Any,
    collections::HashMap,
    fmt::Write,
    fs,
    panic::{self, AssertUnwindSafe},
};

use katex_native::{Color, Font, Item, Layout, PathCommand};
use serde_json::Value;

const CASES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/katex/cases.json");
const LAYOUTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/katex/layouts.txt");

const HEADER: &str = "\
# Layouts of KaTeX's screenshot test cases (tests/katex/cases.json), in ems
# rounded to 0.001. Written by tests/katex_corpus.rs; regenerate with
# UPDATE_LAYOUTS=1 cargo test --test katex_corpus.
#
#   text x y font size \"text\"
#   rect x y width height
#   path commands min-x min-y max-x max-y clip x y width height
#   line x1 y1 x2 y2 thickness
#
# A path's bounds include its control points. A color, if set, ends the line.
";

#[test]
fn katex_screenshot_cases() {
    let cases: Vec<Value> =
        serde_json::from_str(&fs::read_to_string(CASES).expect("read cases")).expect("JSON");
    let ctx = katex_native::katex::KatexContext::default();
    let mut snapshot = HEADER.to_owned();
    let mut panics = Vec::new();
    for case in &cases {
        let name = case["name"].as_str().expect("name");
        let display = case["display"].as_bool().expect("display");
        // Macros become `\newcommand`s, as in examples/corpus.rs.
        let mut tex = case["tex"].as_str().expect("tex").to_owned();
        if let Some(macros) = case["options"]["macros"].as_object() {
            for (macro_name, value) in macros {
                tex = format!(
                    "\\newcommand{{{macro_name}}}{{{}}}{tex}",
                    value.as_str().expect("macro")
                );
            }
        }
        let mode = if display { "display" } else { "inline" };
        writeln!(snapshot, "\n== {name} ({mode})").unwrap();
        match panic::catch_unwind(AssertUnwindSafe(|| {
            katex_native::layout(&ctx, &tex, display)
        })) {
            Ok(Ok(layout)) => write_layout(&mut snapshot, &layout, ""),
            Ok(Err(err)) => {
                writeln!(snapshot, "error {}", err.to_string().replace('\n', "\\n")).unwrap();
            }
            Err(payload) => panics.push(format!("{name}: {}", panic_message(&*payload))),
        }
    }
    assert!(panics.is_empty(), "layout panicked:\n{}", panics.join("\n"));

    if std::env::var_os("UPDATE_LAYOUTS").is_some_and(|update| update == "1") {
        fs::write(LAYOUTS, &snapshot).expect("write layouts");
        return;
    }
    let committed = fs::read_to_string(LAYOUTS).unwrap_or_default();
    if snapshot != committed {
        panic!("{}", describe_changes(&committed, &snapshot));
    }
}

fn write_layout(out: &mut String, layout: &Layout, indent: &str) {
    writeln!(
        out,
        "{indent}width {} height {} depth {}",
        em(layout.width),
        em(layout.height),
        em(layout.depth)
    )
    .unwrap();
    if layout.equation_numbers > 0 {
        writeln!(out, "{indent}equation numbers {}", layout.equation_numbers).unwrap();
    }
    for item in &layout.items {
        let (line, color) = match item {
            Item::Text {
                x,
                y,
                text,
                font,
                size,
                color,
            } => (
                format!(
                    "text {} {} {} {text:?}",
                    ems(&[*x, *y]),
                    font_name(*font),
                    em(*size)
                ),
                color,
            ),
            Item::Rect {
                x,
                y,
                width,
                height,
                color,
            } => (format!("rect {}", ems(&[*x, *y, *width, *height])), color),
            Item::Path {
                commands,
                clip,
                color,
            } => (
                format!(
                    "path {} {} clip {}",
                    commands.len(),
                    ems(&bounds(commands)),
                    ems(&[clip.x, clip.y, clip.width, clip.height])
                ),
                color,
            ),
            Item::Line {
                from,
                to,
                thickness,
                color,
            } => (
                format!("line {}", ems(&[from.0, from.1, to.0, to.1, *thickness])),
                color,
            ),
            other => (format!("{other:?}"), &None),
        };
        writeln!(out, "{indent}{line}{}", color_suffix(*color)).unwrap();
    }
    if let Some(tag) = &layout.tag {
        writeln!(out, "{indent}tag").unwrap();
        write_layout(out, tag, &format!("{indent}  "));
    }
}

/// Ems to 0.001, with no sign on zero.
fn em(value: f64) -> String {
    let rounded = format!("{value:.3}");
    if rounded == "-0.000" {
        "0.000".to_owned()
    } else {
        rounded
    }
}

fn ems(values: &[f64]) -> String {
    values
        .iter()
        .map(|&value| em(value))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The smallest box around a path's points, control points included: min x,
/// min y, max x, max y.
fn bounds(commands: &[PathCommand]) -> [f64; 4] {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut add = |x: f64, y: f64| {
        bounds = [
            bounds[0].min(x),
            bounds[1].min(y),
            bounds[2].max(x),
            bounds[3].max(y),
        ];
    };
    for command in commands {
        match *command {
            PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) => add(x, y),
            PathCommand::CubicTo(x1, y1, x2, y2, x, y) => {
                add(x1, y1);
                add(x2, y2);
                add(x, y);
            }
            PathCommand::Close => {}
        }
    }
    bounds
}

/// `Main`, `Math-Italic`, `Main-BoldItalic` and so on.
fn font_name(font: Font) -> String {
    let style = match (font.bold, font.italic) {
        (false, false) => "",
        (true, false) => "-Bold",
        (false, true) => "-Italic",
        (true, true) => "-BoldItalic",
    };
    let family = font.family.name();
    format!("{}{style}", family.strip_prefix("KaTeX_").unwrap_or(family))
}

fn color_suffix(color: Option<Color>) -> String {
    match color {
        Some(Color { r, g, b, a: 255 }) => format!(" #{r:02x}{g:02x}{b:02x}"),
        Some(Color { r, g, b, a }) => format!(" #{r:02x}{g:02x}{b:02x}{a:02x}"),
        None => String::new(),
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("(not a string)")
}

/// Names the cases whose layout differs from the committed snapshot, with
/// the first line that differs in each.
fn describe_changes(committed: &str, current: &str) -> String {
    let before = split_cases(committed);
    let after = split_cases(current);
    let before_by_name: HashMap<_, _> = before.iter().copied().collect();
    let after_by_name: HashMap<_, _> = after.iter().copied().collect();
    let mut changes = Vec::new();
    for &(name, case) in &after {
        match before_by_name.get(name) {
            None => changes.push(format!("{name}: not in the snapshot")),
            Some(&old) if old != case => {
                let (old_line, new_line) = first_difference(old, case);
                changes.push(format!("{name}:\n  - {old_line}\n  + {new_line}"));
            }
            Some(_) => {}
        }
    }
    for &(name, _) in &before {
        if !after_by_name.contains_key(name) {
            changes.push(format!("{name}: no longer a case"));
        }
    }
    if changes.is_empty() {
        changes.push("the header or line endings differ".to_owned());
    }
    format!(
        "tests/katex/layouts.txt is out of date:\n{}\n\nIf the changes are intended, \
         regenerate it with UPDATE_LAYOUTS=1 cargo test --test katex_corpus and review the diff.",
        changes.join("\n")
    )
}

/// Each case's `== ` line, without the marker, and its section.
fn split_cases(snapshot: &str) -> Vec<(&str, &str)> {
    snapshot
        .split("\n== ")
        .skip(1)
        .map(|case| (case.lines().next().unwrap_or_default(), case))
        .collect()
}

fn first_difference<'a>(old: &'a str, new: &'a str) -> (&'a str, &'a str) {
    let (mut old, mut new) = (old.lines(), new.lines());
    loop {
        match (old.next(), new.next()) {
            (None, None) => return ("(line endings)", "(line endings)"),
            (a, b) if a != b => return (a.unwrap_or("(end)"), b.unwrap_or("(end)")),
            _ => {}
        }
    }
}
