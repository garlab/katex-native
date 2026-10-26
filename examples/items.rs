//! Prints the display list for a TeX expression.
fn main() {
    let tex = std::env::args().nth(1).expect("TeX expression");
    let ctx = katex::KatexContext::default();
    let layout = katex_native::layout(&ctx, &tex, false).expect("layout");
    for item in &layout.items {
        println!("{item:?}");
    }
}
