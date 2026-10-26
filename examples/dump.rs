//! Prints katex-rs's HTML for a TeX expression: `cargo run --example dump -- '\frac{a}{b}' [display]`.

fn main() {
    let mut args = std::env::args().skip(1);
    let tex = args.next().expect("TeX expression");
    let display = args.next().is_some_and(|mode| mode == "display");
    let ctx = katex::KatexContext::default();
    let settings = katex::Settings::builder().display_mode(display).build();
    let html = katex::render_to_string(&ctx, &tex, &settings).expect("render");
    // Only the HTML half; the MathML half is for accessibility.
    let start = html.find("<span class=\"katex-html\"").unwrap_or(0);
    println!("{}", &html[start..]);
}
