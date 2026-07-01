use std::fs;

fn main() {
    let css_text = fs::read_to_string("/home/pedro/Documentos/style.css").unwrap();

    println!("=== Parsing CSS ===");
    let rules = folio_cosmic::css::parse_css(&css_text);

    println!("\n=== Resolving styles ===");

    // Test h1 with class ft1
    println!("\nTesting <h1 class='ft1'>:");
    let style = folio_cosmic::css::resolve_style("h1", &["ft1".to_string()], &rules);
    println!("  font_family: {:?}", style.font_family);
    println!("  font_size: {:?}", style.font_size);

    // Test span with class ft2
    println!("\nTesting <span class='ft2'>:");
    let style = folio_cosmic::css::resolve_style("span", &["ft2".to_string()], &rules);
    println!("  font_family: {:?}", style.font_family);

    // Test p with class ftq
    println!("\nTesting <p class='ftq'>:");
    let style = folio_cosmic::css::resolve_style("p", &["ftq".to_string()], &rules);
    println!("  font_family: {:?}", style.font_family);

    // Test p without class
    println!("\nTesting <p> (no class):");
    let style = folio_cosmic::css::resolve_style("p", &[], &rules);
    println!("  font_family: {:?}", style.font_family);
}
