//! Applies KaTeX's stylesheet (`katex.css` from KaTeX 0.18.5, the version
//! katex-rs 0.3 tracks) to katex-rs's HTML tree.
//!
//! The CSS only needs a small box model: inline content flows left to right
//! on a shared baseline, and KaTeX's vertical lists (`vlist-t`) already say
//! where each row's baseline goes. Each node becomes a [`Boxed`]: its items
//! relative to the left end of its baseline, with y pointing down.

use katex::{
    ClassList, KatexContext,
    dom_tree::{DomSpan, HtmlDomNode, LineNode, SvgChildNode, SvgNode, SymbolNode},
    svg_geometry::PATH_MAP,
    symbols::Mode,
    types::{CssProperty, CssStyle},
};

use crate::{Color, Error, Family, Font, Item, Layout, Rect, font_data, path};

/// KaTeX's font sizes `size1` to `size11`, relative to the default `size6`.
const SIZES: [f64; 11] = [0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.2, 1.44, 1.728, 2.074, 2.488];

pub(crate) fn layout(katex: &KatexContext, tree: &DomSpan, display: bool) -> Result<Layout, Error> {
    let engine = Engine { katex };
    let ctx = Ctx {
        font: Font::default(),
        scale: 1.,
        color: None,
        // `.katex-display > .katex` centers, and text-align inherits.
        align: if display { Align::Center } else { Align::Left },
    };
    let html = find_class(tree, "katex-html").unwrap_or(tree);
    let mut lines = vec![Boxed::default()];
    let mut tag = None;
    for child in &html.children {
        if let HtmlDomNode::DomSpan(span) = child {
            if span.classes.contains("katex-tag") {
                tag = Some(Box::new(engine.span(span, ctx, None)?.into_layout()));
                continue;
            }
            // `\\` outside an environment: a block that starts a new line.
            if span.classes.contains("katex-newline") {
                lines.push(Boxed::default());
                continue;
            }
        }
        let line = lines.last_mut().expect("at least one line");
        line.append(engine.node(child, ctx, None)?);
    }
    let mut layout = stack_lines(lines, ctx.align).into_layout();
    layout.tag = tag;
    Ok(layout)
}

/// Stacks lines with the first one's baseline at 0, as a browser stacks line
/// boxes: each is at least as tall as `KaTeX_Main` at `line-height: 1.2`
/// (its ascent and descent plus half the leading on each side).
fn stack_lines(mut lines: Vec<Boxed>, align: Align) -> Boxed {
    if lines.len() == 1 {
        return lines.pop().expect("one line");
    }
    let leading = (1.2 - font_data::MAIN_ASCENT - font_data::MAIN_DESCENT) / 2.;
    let (min_above, min_below) = (
        font_data::MAIN_ASCENT + leading,
        font_data::MAIN_DESCENT + leading,
    );
    let width = lines.iter().map(|line| line.width).fold(0., f64::max);
    let mut stacked = Boxed {
        width,
        ..Boxed::default()
    };
    let mut baseline = 0.;
    let mut previous_below = None;
    for mut line in lines {
        match previous_below {
            Some(below) => baseline += below + line.height.max(min_above),
            None => stacked.height = line.height,
        }
        previous_below = Some(line.depth.max(min_below));
        let free = width - line.width;
        let dx = match align {
            Align::Left => 0.,
            Align::Center => free / 2.,
            Align::Right => free,
        };
        stacked.depth = baseline + line.depth;
        line.translate(dx, baseline);
        stacked.items.append(&mut line.items);
    }
    stacked
}

fn find_class<'a>(span: &'a DomSpan, class: &str) -> Option<&'a DomSpan> {
    if span.classes.contains(class) {
        return Some(span);
    }
    span.children.iter().find_map(|child| match child {
        HtmlDomNode::DomSpan(child) => find_class(child, class),
        _ => None,
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Align {
    Left,
    Center,
    Right,
}

/// The inherited CSS properties that matter for layout.
#[derive(Clone, Copy)]
struct Ctx {
    font: Font,
    /// Font size relative to the formula's.
    scale: f64,
    color: Option<Color>,
    /// `text-align`, which positions rows inside vertical lists.
    align: Align,
}

impl Ctx {
    /// Applies the rules `classes` and `style` select, in katex.css order.
    fn apply(mut self, classes: &ClassList, style: &CssStyle) -> Self {
        let has = |class: &str| classes.contains(class);
        let font = &mut self.font;
        if has("textbf") {
            font.bold = true;
        }
        if has("textit") {
            font.italic = true;
        }
        if has("textrm") {
            font.family = Family::Main;
        }
        if has("textsf") {
            font.family = Family::SansSerif;
        }
        if has("texttt") {
            font.family = Family::Typewriter;
        }
        if has("mathnormal") {
            font.family = Family::Math;
            font.italic = true;
        }
        if has("mathit") {
            font.family = Family::Main;
            font.italic = true;
        }
        if has("mathrm") {
            font.italic = false;
        }
        if has("mathbf") {
            font.family = Family::Main;
            font.bold = true;
        }
        if has("boldsymbol") {
            font.family = Family::Math;
            font.bold = true;
            font.italic = true;
        }
        if has("amsrm") || has("mathbb") || has("textbb") {
            font.family = Family::Ams;
        }
        if has("mathcal") {
            font.family = Family::Caligraphic;
        }
        if has("mathfrak") || has("textfrak") {
            font.family = Family::Fraktur;
        }
        if has("mathboldfrak") || has("textboldfrak") {
            font.family = Family::Fraktur;
            font.bold = true;
        }
        if has("mathtt") {
            font.family = Family::Typewriter;
        }
        if has("mathscr") || has("textscr") {
            font.family = Family::Script;
        }
        if has("mathsf") || has("textsf") {
            font.family = Family::SansSerif;
        }
        if has("mathboldsf") || has("textboldsf") {
            font.family = Family::SansSerif;
            font.bold = true;
        }
        if has("mathsfit") || has("mathitsf") || has("textitsf") {
            font.family = Family::SansSerif;
            font.italic = true;
        }
        if has("mainrm") {
            font.family = Family::Main;
            font.italic = false;
        }
        if has("delimsizing") {
            for (class, size) in [
                ("size1", Family::Size1),
                ("size2", Family::Size2),
                ("size3", Family::Size3),
                ("size4", Family::Size4),
            ] {
                if has(class) {
                    font.family = size;
                }
            }
        }
        // `.delimsizing.mult .delim-size1 > span`, which only has text children.
        if has("delim-size1") {
            font.family = Family::Size1;
        }
        if has("delim-size4") {
            font.family = Family::Size4;
        }
        if has("op-symbol") {
            if has("small-op") {
                font.family = Family::Size1;
            }
            if has("large-op") {
                font.family = Family::Size2;
            }
        }

        if has("katex-sizing") || has("fontsize-ensurer") {
            let size = |prefix: &str| {
                classes.iter().find_map(|class| {
                    let n: usize = class.strip_prefix(prefix)?.parse().ok()?;
                    SIZES.get(n.checked_sub(1)?).copied()
                })
            };
            if let (Some(from), Some(to)) = (size("reset-size"), size("size")) {
                self.scale *= to / from;
            }
        }

        for (class, align) in [
            ("msupsub", Align::Left),
            ("mfrac", Align::Center),
            ("op-limits", Align::Center),
            ("katex-accent", Align::Center),
            ("col-align-c", Align::Center),
            ("col-align-l", Align::Left),
            ("col-align-r", Align::Right),
            ("svg-align", Align::Left),
            ("x-arrow", Align::Center),
            ("mover", Align::Center),
            ("munder", Align::Center),
        ] {
            if has(class) {
                self.align = align;
            }
        }

        if let Some(color) = style.get(CssProperty::Color).and_then(Color::parse) {
            self.color = Some(color);
        }
        self
    }

    /// A length in this element's ems, as a fraction of the formula's em.
    fn em(&self, style: &CssStyle, property: CssProperty) -> Option<f64> {
        parse_em(style.get(property)?).map(|em| em * self.scale)
    }
}

/// Parses `1.5em`; other units don't occur in KaTeX's layout styles.
fn parse_em(value: &str) -> Option<f64> {
    value.trim().strip_suffix("em")?.trim().parse().ok()
}

/// A laid-out node.
#[derive(Default)]
struct Boxed {
    width: f64,
    height: f64,
    depth: f64,
    items: Vec<Item>,
    /// Whether some part is as wide as its container (`width: 100%`), so the
    /// node must be laid out again once that width is known.
    fills: bool,
}

impl Boxed {
    /// Appends `next` on the same baseline.
    fn append(&mut self, mut next: Boxed) {
        next.translate(self.width, 0.);
        self.width += next.width;
        self.height = self.height.max(next.height);
        self.depth = self.depth.max(next.depth);
        self.items.append(&mut next.items);
        self.fills |= next.fills;
    }

    /// Moves the contents right by `dx` and down by `dy`.
    fn translate(&mut self, dx: f64, dy: f64) {
        self.height -= dy;
        self.depth += dy;
        if dx == 0. && dy == 0. {
            return;
        }
        for item in &mut self.items {
            match item {
                Item::Text { x, y, .. } | Item::Rect { x, y, .. } => {
                    *x += dx;
                    *y += dy;
                }
                Item::Path { commands, clip, .. } => {
                    clip.x += dx;
                    clip.y += dy;
                    for command in commands {
                        match command {
                            path::PathCommand::MoveTo(x, y) | path::PathCommand::LineTo(x, y) => {
                                *x += dx;
                                *y += dy;
                            }
                            path::PathCommand::CubicTo(a, b, c, d, e, f) => {
                                *a += dx;
                                *b += dy;
                                *c += dx;
                                *d += dy;
                                *e += dx;
                                *f += dy;
                            }
                            path::PathCommand::Close => {}
                        }
                    }
                }
                Item::Line { from, to, .. } => {
                    from.0 += dx;
                    from.1 += dy;
                    to.0 += dx;
                    to.1 += dy;
                }
            }
        }
    }

    fn into_layout(self) -> Layout {
        Layout {
            width: self.width,
            height: self.height,
            depth: self.depth,
            items: self.items,
            tag: None,
        }
    }
}

/// Classes whose elements are as wide as their container.
const FILLS: &[&str] = &[
    "frac-line",
    "overline-line",
    "underline-line",
    "katex-hline",
    "katex-hdashline",
    "hide-tail",
    "katex-stretchy",
];

struct Engine<'a> {
    katex: &'a KatexContext,
}

impl Engine<'_> {
    /// `avail` is the width of the containing block, once known.
    fn node(&self, node: &HtmlDomNode, ctx: Ctx, avail: Option<f64>) -> Result<Boxed, Error> {
        match node {
            HtmlDomNode::DomSpan(span) => self.span(span, ctx, avail),
            HtmlDomNode::Anchor(anchor) => {
                let ctx = ctx.apply(&anchor.classes, &anchor.style);
                self.hlist(&anchor.children, ctx, avail)
            }
            HtmlDomNode::Symbol(symbol) => Ok(self.symbol(symbol, ctx)),
            HtmlDomNode::Fragment(fragment) => self.hlist(&fragment.children, ctx, avail),
            HtmlDomNode::Img(img) => {
                // Images aren't drawn, but keep their space.
                let ctx = ctx.apply(&img.classes, &img.style);
                Ok(Boxed {
                    width: ctx.em(&img.style, CssProperty::Width).unwrap_or(0.),
                    height: ctx.em(&img.style, CssProperty::Height).unwrap_or(0.),
                    ..Boxed::default()
                })
            }
            HtmlDomNode::SvgNode(svg) => {
                let width = svg_length(svg, "width").unwrap_or(0.) * ctx.scale;
                let height = svg_length(svg, "height").unwrap_or(0.) * ctx.scale;
                let mut boxed = Boxed {
                    width,
                    height,
                    ..Boxed::default()
                };
                draw_svg(
                    svg,
                    ctx,
                    Rect {
                        x: 0.,
                        y: -height,
                        width,
                        height,
                    },
                    &mut boxed.items,
                );
                Ok(boxed)
            }
            HtmlDomNode::MathML(_) => Ok(Boxed::default()),
        }
    }

    fn hlist(&self, nodes: &[HtmlDomNode], ctx: Ctx, avail: Option<f64>) -> Result<Boxed, Error> {
        let mut boxed = Boxed::default();
        for node in nodes {
            boxed.append(self.node(node, ctx, avail)?);
        }
        Ok(boxed)
    }

    fn symbol(&self, symbol: &SymbolNode, ctx: Ctx) -> Boxed {
        let ctx = ctx.apply(&symbol.classes, &symbol.style);
        let width: f64 = symbol
            .text
            .chars()
            .map(|ch| self.advance(ch, ctx.font))
            .sum::<f64>()
            * ctx.scale;
        let content = Boxed {
            width,
            height: symbol.height * ctx.scale,
            depth: symbol.depth * ctx.scale,
            items: if symbol.text.trim_matches('\u{200b}').is_empty() {
                Vec::new()
            } else {
                vec![Item::Text {
                    x: 0.,
                    y: 0.,
                    text: symbol.text.clone(),
                    font: ctx.font,
                    size: ctx.scale,
                    color: ctx.color,
                }]
            },
            fills: false,
        };
        // KaTeX writes a symbol's italic correction as its right margin.
        let italic = if symbol.italic > 0. {
            symbol.italic * ctx.scale
        } else {
            0.
        };
        decorate(content, &symbol.classes, &symbol.style, ctx, italic)
    }

    /// The advance width of `ch` in `font`, in ems: from the font file, as
    /// a browser measures it, else from KaTeX's metrics.
    fn advance(&self, ch: char, font: Font) -> f64 {
        if ch == '\u{200b}' {
            return 0.;
        }
        font_data::advance(font, ch)
            .or_else(|| {
                [metric_name(font), "Main-Regular"]
                    .into_iter()
                    .find_map(|name| {
                        katex::get_character_metrics(self.katex, ch, name, Mode::Text)
                            .ok()
                            .flatten()
                            .map(|metrics| metrics.width)
                    })
            })
            // Browsers draw such characters from a fallback font; assume a
            // typical width.
            .unwrap_or(0.6)
    }

    fn span(&self, span: &DomSpan, ctx: Ctx, avail: Option<f64>) -> Result<Boxed, Error> {
        let classes = &span.classes;
        let style = &span.style;
        if classes.contains("katex-mathml") || classes.contains("vlist-s") {
            return Ok(Boxed::default());
        }
        let ctx = ctx.apply(classes, style);
        if classes.contains("vlist-t") {
            return Ok(decorate(self.vlist(span, ctx)?, classes, style, ctx, 0.));
        }
        if classes.contains("katex-strut") {
            let height = ctx.em(style, CssProperty::Height).unwrap_or(0.);
            let shift = ctx.em(style, CssProperty::VerticalAlign).unwrap_or(0.);
            return Ok(Boxed {
                height: height + shift,
                depth: -shift,
                ..Boxed::default()
            });
        }
        if [
            "frac-line",
            "overline-line",
            "underline-line",
            "katex-hline",
            "katex-hdashline",
        ]
        .iter()
        .any(|class| classes.contains(class))
        {
            // An empty inline-block whose bottom border is the line.
            let thickness = ctx
                .em(style, CssProperty::BorderBottomWidth)
                .unwrap_or(0.04 * ctx.scale);
            let width = avail.unwrap_or(0.);
            let rule = Rect {
                x: 0.,
                y: -thickness,
                width,
                height: thickness,
            };
            let color = border_color(style).or(ctx.color);
            return Ok(Boxed {
                width,
                height: thickness,
                depth: 0.,
                items: if classes.contains("katex-hdashline") {
                    dashes(rule, false, color)
                } else {
                    vec![rect(rule, color)]
                },
                fills: true,
            });
        }
        if classes.contains("katex-rule") {
            let width = ctx.em(style, CssProperty::BorderRightWidth).unwrap_or(0.);
            let height = ctx.em(style, CssProperty::BorderTopWidth).unwrap_or(0.);
            let raise = ctx.em(style, CssProperty::Bottom).unwrap_or(0.);
            return Ok(decorate_margins(
                Boxed {
                    width,
                    height: height + raise,
                    depth: -raise,
                    items: vec![Item::Rect {
                        x: 0.,
                        y: -(height + raise),
                        width,
                        height,
                        color: border_color(style).or(ctx.color),
                    }],
                    fills: false,
                },
                style,
                ctx,
            ));
        }
        if span
            .children
            .iter()
            .any(|child| matches!(child, HtmlDomNode::SvgNode(_)))
            || classes.contains("katex-stretchy")
        {
            return Ok(self.svg_container(span, ctx, avail));
        }
        if ["llap", "rlap", "clap"]
            .iter()
            .any(|class| classes.contains(class))
        {
            return self.lap(span, ctx, avail);
        }

        let mut content = self.hlist(&span.children, ctx, avail)?;
        if classes.contains("nulldelimiter") {
            content.width = 0.12 * ctx.scale;
        }
        if classes.contains("accent-body") && !classes.contains("accent-full") {
            content.width = 0.;
        }
        if let Some(width) = ctx.em(style, CssProperty::Width) {
            content.width = width;
        }
        if span.children.is_empty()
            && let Some(height) = ctx.em(style, CssProperty::Height)
        {
            content.height = content.height.max(height);
        }
        Ok(decorate(content, classes, style, ctx, 0.))
    }

    /// A KaTeX vertical list: `vlist-t > vlist-r > vlist > row`, where each
    /// row is `[pstrut, content]` shifted by `top`. With a strut `p` ems
    /// tall, the content's baseline ends up `-top - p` ems above the list's.
    fn vlist(&self, span: &DomSpan, ctx: Ctx) -> Result<Boxed, Error> {
        fn cell(row: Option<&DomSpan>) -> Option<&DomSpan> {
            spans(&row?.children).find(|cell| cell.classes.contains("vlist"))
        }
        let rows: Vec<&DomSpan> = spans(&span.children).collect();
        let first =
            cell(rows.first().copied()).ok_or_else(|| Error::Unsupported("empty vlist".into()))?;
        let height = ctx.em(&first.style, CssProperty::Height).unwrap_or(0.);
        let depth = cell(rows.get(1).copied())
            .and_then(|cell| ctx.em(&cell.style, CssProperty::Height))
            .unwrap_or(0.);

        struct Row<'a> {
            content: Boxed,
            baseline: f64,
            margins: (f64, f64),
            ctx: Ctx,
            children: &'a [HtmlDomNode],
        }
        let mut laid_out = Vec::new();
        for row in spans(&first.children) {
            let ctx = ctx.apply(&row.classes, &row.style);
            let (strut, children) = match row.children.split_first() {
                Some((HtmlDomNode::DomSpan(strut), rest)) if strut.classes.contains("pstrut") => (
                    ctx.em(&strut.style, CssProperty::Height).unwrap_or(0.),
                    rest,
                ),
                _ => (0., &row.children[..]),
            };
            let top = ctx.em(&row.style, CssProperty::Top).unwrap_or(0.);
            laid_out.push(Row {
                content: self.hlist(children, ctx, None)?,
                baseline: -top - strut,
                margins: (
                    ctx.em(&row.style, CssProperty::MarginLeft).unwrap_or(0.),
                    ctx.em(&row.style, CssProperty::MarginRight).unwrap_or(0.),
                ),
                ctx,
                children,
            });
        }
        let width = laid_out
            .iter()
            .map(|row| row.margins.0 + row.content.width + row.margins.1)
            .fold(0., f64::max);
        let mut boxed = Boxed {
            width,
            height,
            depth,
            ..Boxed::default()
        };
        for mut row in laid_out {
            let inner = width - row.margins.0 - row.margins.1;
            if row.content.fills {
                row.content = self.hlist(row.children, row.ctx, Some(inner))?;
            }
            let free = inner - row.content.width;
            let dx = row.margins.0
                + match row.ctx.align {
                    Align::Left => 0.,
                    Align::Center => free / 2.,
                    Align::Right => free,
                };
            row.content.translate(dx, -row.baseline);
            boxed.items.append(&mut row.content.items);
        }
        Ok(boxed)
    }

    /// `llap`, `rlap` and `clap`: zero-width boxes whose `katex-inner` child
    /// hangs left, right or centered.
    fn lap(&self, span: &DomSpan, ctx: Ctx, avail: Option<f64>) -> Result<Boxed, Error> {
        let mut boxed = Boxed::default();
        for child in spans(&span.children) {
            let inner_ctx = ctx.apply(&child.classes, &child.style);
            let mut inner = self.hlist(&child.children, inner_ctx, avail)?;
            if !child.classes.contains("katex-inner") {
                // `katex-fix` only holds a zero-width space.
                continue;
            }
            let dx = if span.classes.contains("llap") {
                -inner.width
            } else if span.classes.contains("clap") {
                -inner.width / 2.
            } else {
                0.
            };
            inner.translate(dx, 0.);
            boxed.height = boxed.height.max(inner.height);
            boxed.depth = boxed.depth.max(inner.depth);
            boxed.items.append(&mut inner.items);
        }
        Ok(decorate(boxed, &span.classes, &span.style, ctx, 0.))
    }

    /// A span sized by its style whose SVG children fill it: square-root
    /// signs, stretchy arrows and braces, tall delimiters.
    fn svg_container(&self, span: &DomSpan, ctx: Ctx, avail: Option<f64>) -> Boxed {
        let classes = &span.classes;
        let style = &span.style;
        let first_svg = span.children.iter().find_map(|child| match child {
            HtmlDomNode::SvgNode(svg) => Some(svg),
            _ => None,
        });
        // KaTeX's CSS makes every SVG as wide as its nearest positioned
        // ancestor. In a vertical list row (`\cancel`), that's the row.
        let fills = FILLS.iter().any(|class| classes.contains(class))
            || first_svg.is_some_and(|svg| {
                svg.attributes
                    .get("width")
                    .is_some_and(|width| width.ends_with('%'))
            });
        // Until the container's width is known, a filling shape takes none.
        let width = ctx
            .em(style, CssProperty::Width)
            .or_else(|| {
                if fills {
                    Some(avail.unwrap_or(0.))
                } else {
                    first_svg
                        .and_then(|svg| svg_length(svg, "width"))
                        .map(|w| w * ctx.scale)
                }
            })
            .unwrap_or(0.)
            .max(ctx.em(style, CssProperty::MinWidth).unwrap_or(0.));
        let height = ctx
            .em(style, CssProperty::Height)
            .or_else(|| {
                first_svg
                    .and_then(|svg| svg_length(svg, "height"))
                    .map(|h| h * ctx.scale)
            })
            .unwrap_or(0.);
        let rect = Rect {
            x: 0.,
            y: -height,
            width,
            height,
        };
        let mut items = Vec::new();
        for child in &span.children {
            match child {
                HtmlDomNode::SvgNode(svg) => draw_svg(svg, ctx, rect, &mut items),
                // Absolutely positioned pieces of a stretchy shape.
                HtmlDomNode::DomSpan(piece) => {
                    let pieces = [
                        ("halfarrow-left", 0., 0.502),
                        ("halfarrow-right", 1. - 0.502, 0.502),
                        ("brace-left", 0., 0.251),
                        ("brace-center", 0.25, 0.5),
                        ("brace-right", 1. - 0.251, 0.251),
                    ];
                    let Some(&(_, left, share)) = pieces
                        .iter()
                        .find(|(class, ..)| piece.classes.contains(class))
                    else {
                        continue;
                    };
                    let piece_ctx = ctx.apply(&piece.classes, &piece.style);
                    let piece_height = piece_ctx
                        .em(&piece.style, CssProperty::Height)
                        .unwrap_or(height);
                    let piece_rect = Rect {
                        x: width * left,
                        y: -height,
                        width: width * share,
                        height: piece_height,
                    };
                    for grandchild in &piece.children {
                        if let HtmlDomNode::SvgNode(svg) = grandchild {
                            draw_svg(svg, piece_ctx, piece_rect, &mut items);
                        }
                    }
                }
                _ => {}
            }
        }
        let boxed = Boxed {
            width,
            height,
            depth: 0.,
            items,
            fills,
        };
        decorate(boxed, classes, style, ctx, 0.)
    }
}

fn spans(nodes: &[HtmlDomNode]) -> impl Iterator<Item = &DomSpan> {
    nodes.iter().filter_map(|node| match node {
        HtmlDomNode::DomSpan(span) => Some(span),
        _ => None,
    })
}

/// Margins, padding, relative positioning, backgrounds and borders.
/// `extra_right` adds to the right margin (italic corrections).
fn decorate(
    mut boxed: Boxed,
    classes: &ClassList,
    style: &CssStyle,
    ctx: Ctx,
    extra_right: f64,
) -> Boxed {
    let has = |class: &str| classes.contains(class);
    let (mut pad_left, pad_right) = [
        ("x-arrow-pad", 0.5, 0.5),
        ("cd-arrow-pad", 0.27778, 0.55556),
        ("boxpad", 0.3, 0.3),
        ("cancel-pad", 0.2, 0.2),
        ("anglpad", 0.03889, 0.03889),
    ]
    .iter()
    .find(|(class, ..)| has(class))
    .map_or((0., 0.), |&(_, left, right)| {
        (left * ctx.scale, right * ctx.scale)
    });
    if let Some(left) = ctx.em(style, CssProperty::PaddingLeft) {
        pad_left = left;
    }
    boxed.translate(pad_left, 0.);
    boxed.width += pad_left + pad_right;

    // Borders drawn around the padding box.
    let border = if has("fbox") || has("fcolorbox") {
        Some(
            ctx.em(style, CssProperty::BorderWidth)
                .unwrap_or(0.04 * ctx.scale),
        )
    } else {
        None
    };
    let border_color = border_color(style).or(ctx.color);
    if let Some(color) = style
        .get(CssProperty::BackgroundColor)
        .and_then(Color::parse)
    {
        boxed.items.insert(
            0,
            Item::Rect {
                x: 0.,
                y: -boxed.height,
                width: boxed.width,
                height: boxed.height + boxed.depth,
                color: Some(color),
            },
        );
    }
    if let Some(thickness) = border {
        let (w, top, bottom) = (boxed.width, -boxed.height, boxed.depth);
        for (x, y, width, height) in [
            (0., top, w, thickness),
            (0., bottom - thickness, w, thickness),
            (0., top, thickness, bottom - top),
            (w - thickness, top, thickness, bottom - top),
        ] {
            boxed.items.push(Item::Rect {
                x,
                y,
                width,
                height,
                color: border_color,
            });
        }
    }
    if has("angl") {
        let thickness = 0.049 * ctx.scale;
        let (w, top, bottom) = (boxed.width, -boxed.height, boxed.depth);
        boxed.items.push(Item::Rect {
            x: 0.,
            y: top,
            width: w,
            height: thickness,
            color: border_color,
        });
        boxed.items.push(Item::Rect {
            x: w - thickness,
            y: top,
            width: thickness,
            height: bottom - top,
            color: border_color,
        });
        boxed.width += 0.03889 * ctx.scale;
    }
    if has("katex-sout") {
        let thickness = 0.08 * ctx.scale;
        boxed.items.push(Item::Rect {
            x: 0.,
            y: boxed.depth - thickness,
            width: boxed.width,
            height: thickness,
            color: border_color,
        });
    }
    if has("vertical-separator") {
        let thickness = ctx
            .em(style, CssProperty::BorderRightWidth)
            .unwrap_or(0.04 * ctx.scale);
        let (top, bottom) = (-boxed.height, boxed.depth);
        let rule = Rect {
            x: boxed.width,
            y: top,
            width: thickness,
            height: bottom - top,
        };
        if style.get(CssProperty::BorderRightStyle) == Some("dashed") {
            boxed.items.extend(dashes(rule, true, border_color));
        } else {
            boxed.items.push(rect(rule, border_color));
        }
        boxed.width += thickness;
    }

    // Relative positioning moves the box without changing its space.
    let dx = ctx.em(style, CssProperty::Left).unwrap_or(0.);
    let dy = ctx.em(style, CssProperty::Top).unwrap_or(0.)
        - ctx.em(style, CssProperty::Bottom).unwrap_or(0.)
        - ctx.em(style, CssProperty::VerticalAlign).unwrap_or(0.);
    if dx != 0. || dy != 0. {
        let (height, depth) = (boxed.height, boxed.depth);
        boxed.translate(dx, dy);
        if style.get(CssProperty::VerticalAlign).is_none() {
            // `top`/`bottom` don't affect the line's height.
            (boxed.height, boxed.depth) = (height, depth);
        }
    }

    let mut boxed = decorate_margins(boxed, style, ctx);
    let (mut left, mut right) = (0., extra_right);
    if has("katex-root") {
        left += 0.2777777778 * ctx.scale;
        right -= 0.5555555556 * ctx.scale;
    }
    if has("cancel-lap") {
        left -= 0.2 * ctx.scale;
        right -= 0.2 * ctx.scale;
    }
    boxed.translate(left, 0.);
    boxed.width += left + right;
    boxed
}

fn decorate_margins(mut boxed: Boxed, style: &CssStyle, ctx: Ctx) -> Boxed {
    // The `margin` shorthand: top, right, bottom, left, with omitted sides
    // copied from their opposites.
    let shorthand: Vec<f64> = style
        .get(CssProperty::Margin)
        .map(|value| {
            value
                .split_whitespace()
                .map(|part| {
                    if part == "0" {
                        Some(0.)
                    } else {
                        parse_em(part)
                    }
                })
                .map(|em| em.unwrap_or(0.) * ctx.scale)
                .collect()
        })
        .unwrap_or_default();
    let (short_right, short_left) = match shorthand[..] {
        [all] => (all, all),
        [_, sides] | [_, sides, _] => (sides, sides),
        [_, right, _, left, ..] => (right, left),
        [] => (0., 0.),
    };
    let left = ctx.em(style, CssProperty::MarginLeft).unwrap_or(short_left);
    let right = ctx
        .em(style, CssProperty::MarginRight)
        .unwrap_or(short_right);
    boxed.translate(left, 0.);
    boxed.width += left + right;
    boxed
}

fn rect(rect: Rect, color: Option<Color>) -> Item {
    Item::Rect {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        color,
    }
}

/// A dashed border along `rule`, as Chrome draws one: dashes twice the
/// thickness, gaps once.
fn dashes(rule: Rect, vertical: bool, color: Option<Color>) -> Vec<Item> {
    let (length, thickness) = if vertical {
        (rule.height, rule.width)
    } else {
        (rule.width, rule.height)
    };
    if thickness <= 0. {
        return Vec::new();
    }
    let mut items = Vec::new();
    let mut at = 0.;
    while at < length {
        let dash = (2. * thickness).min(length - at);
        items.push(rect(
            if vertical {
                Rect {
                    y: rule.y + at,
                    height: dash,
                    ..rule
                }
            } else {
                Rect {
                    x: rule.x + at,
                    width: dash,
                    ..rule
                }
            },
            color,
        ));
        at += 3. * thickness;
    }
    items
}

fn border_color(style: &CssStyle) -> Option<Color> {
    style.get(CssProperty::BorderColor).and_then(Color::parse)
}

fn metric_name(font: Font) -> &'static str {
    match (font.family, font.bold, font.italic) {
        (Family::Main, false, false) => "Main-Regular",
        (Family::Main, true, false) => "Main-Bold",
        (Family::Main, false, true) => "Main-Italic",
        (Family::Main, true, true) => "Main-BoldItalic",
        (Family::Math, true, _) => "Math-BoldItalic",
        (Family::Math, false, _) => "Math-Italic",
        (Family::Ams, ..) => "AMS-Regular",
        (Family::Caligraphic, ..) => "Caligraphic-Regular",
        (Family::Fraktur, ..) => "Fraktur-Regular",
        (Family::SansSerif, true, _) => "SansSerif-Bold",
        (Family::SansSerif, false, true) => "SansSerif-Italic",
        (Family::SansSerif, false, false) => "SansSerif-Regular",
        (Family::Script, ..) => "Script-Regular",
        (Family::Typewriter, ..) => "Typewriter-Regular",
        (Family::Size1, ..) => "Size1-Regular",
        (Family::Size2, ..) => "Size2-Regular",
        (Family::Size3, ..) => "Size3-Regular",
        (Family::Size4, ..) => "Size4-Regular",
    }
}

/// An SVG length attribute in ems (`1.08em`), or a bare number.
fn svg_length(svg: &SvgNode, name: &str) -> Option<f64> {
    let value = svg.attributes.get(name)?;
    parse_em(value).or_else(|| value.trim().parse().ok())
}

/// Draws `svg`'s paths and lines into `rect`, as `width: 100%; height:
/// inherit` makes a browser do, honoring `viewBox` and
/// `preserveAspectRatio`, and clipped to `rect`.
fn draw_svg(svg: &SvgNode, ctx: Ctx, rect: Rect, items: &mut Vec<Item>) {
    let view_box: Option<[f64; 4]> = svg.attributes.get("viewBox").and_then(|value| {
        let numbers: Vec<f64> = value
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse().ok())
            .collect();
        numbers.try_into().ok()
    });
    let aspect = svg
        .attributes
        .get("preserveAspectRatio")
        .map_or("xMidYMid meet", String::as_str);
    // Maps view box coordinates to ems.
    let (sx, sy, tx, ty) = match view_box {
        Some([vx, vy, vw, vh]) if vw > 0. && vh > 0. => {
            let (fx, fy) = (rect.width / vw, rect.height / vh);
            if aspect.trim() == "none" {
                (fx, fy, rect.x - vx * fx, rect.y - vy * fy)
            } else {
                let s = if aspect.contains("slice") {
                    fx.max(fy)
                } else {
                    fx.min(fy)
                };
                let offset = |free: f64, min: &str, max: &str| {
                    if aspect.contains(min) {
                        0.
                    } else if aspect.contains(max) {
                        free
                    } else {
                        free / 2.
                    }
                };
                let x = rect.x + offset(rect.width - vw * s, "xMin", "xMax") - vx * s;
                let y = rect.y + offset(rect.height - vh * s, "YMin", "YMax") - vy * s;
                (s, s, x, y)
            }
        }
        // Without a view box, SVG units are CSS pixels; KaTeX only uses
        // percentages there.
        _ => (1., 1., rect.x, rect.y),
    };
    for child in &svg.children {
        match child {
            SvgChildNode::Path(node) => {
                let data = node
                    .alternate
                    .as_deref()
                    .or_else(|| PATH_MAP.get(node.path_name.as_str()).copied())
                    .unwrap_or_default();
                let commands = path::parse(data, |x, y| (tx + x * sx, ty + y * sy));
                if !commands.is_empty() {
                    items.push(Item::Path {
                        commands,
                        clip: rect,
                        color: ctx.color,
                    });
                }
            }
            SvgChildNode::Line(line) => {
                if let Some(item) = svg_line(line, ctx, rect) {
                    items.push(item);
                }
            }
        }
    }
}

/// A `<line>` whose coordinates are percentages of `rect` (KaTeX's
/// `\cancel` strokes).
fn svg_line(line: &LineNode, ctx: Ctx, rect: Rect) -> Option<Item> {
    let coordinate = |name: &str, origin: f64, extent: f64| -> Option<f64> {
        let value = line.attributes.get(name)?.trim();
        Some(match value.strip_suffix('%') {
            Some(percent) => origin + percent.parse::<f64>().ok()? / 100. * extent,
            None => {
                origin
                    + parse_em(value)
                        .map_or_else(|| value.parse().ok(), |em| Some(em * ctx.scale))?
            }
        })
    };
    let thickness = line
        .attributes
        .get("stroke-width")
        .and_then(|value| parse_em(value))
        .unwrap_or(0.046)
        * ctx.scale;
    Some(Item::Line {
        from: (
            coordinate("x1", rect.x, rect.width)?,
            coordinate("y1", rect.y, rect.height)?,
        ),
        to: (
            coordinate("x2", rect.x, rect.width)?,
            coordinate("y2", rect.y, rect.height)?,
        ),
        thickness,
        color: ctx.color,
    })
}
