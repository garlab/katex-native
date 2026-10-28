//! Prints the display list for a TeX expression:
//! `cargo run --example items -- '\frac{a}{b}' [display]`.
fn main() {
    let mut args = std::env::args().skip(1);
    let tex = args.next().expect("TeX expression");
    let display = args.next().is_some_and(|mode| mode == "display");
    let ctx = katex::KatexContext::default();
    let layout = katex_native::layout(&ctx, &tex, display).expect("layout");
    for item in &layout.items {
        println!("{item:?}");
    }
    if let Some(tag) = &layout.tag {
        println!("tag:");
        for item in &tag.items {
            println!("  {item:?}");
        }
    }
    println!("equation numbers used: {}", layout.equation_numbers);
}
