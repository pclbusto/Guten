use folio_cosmic::document::DocumentModel;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let path = PathBuf::from("/home/pedro/Descargas/Jaula de dragones - Andrea Izquierdo.epub");
    if !path.exists() {
        eprintln!("EPUB not found at {:?}", path);
        return;
    }

    eprintln!("Opening {:?}...", path);
    let start = Instant::now();
    let mut doc = match DocumentModel::open(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };
    eprintln!("Opened in {:.2}s", start.elapsed().as_secs_f64());

    let first = doc.find_first_content_chapter();
    doc.goto_spine_index(first);
    eprintln!("Chapter: spine[{}]", first);

    let html = match doc.current_chapter_html() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error reading: {}", e);
            return;
        }
    };
    eprintln!("HTML: {} bytes", html.len());

    eprintln!("Parsing...");
    let parse_start = Instant::now();
    let blocks = folio_cosmic::content::parse_xhtml(&html);
    eprintln!(
        "Parsed {} blocks in {:.2}ms",
        blocks.len(),
        parse_start.elapsed().as_secs_f64() * 1000.0
    );

    for (i, block) in blocks.iter().enumerate() {
        match block {
            folio_cosmic::content::ContentBlock::Heading { level, spans } => {
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                eprintln!("  [{}] H{}: {:?}", i, level, &text[..text.len().min(60)]);
            }
            folio_cosmic::content::ContentBlock::Paragraph { spans } => {
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                eprintln!("  [{}] P: {:?}", i, &text[..text.len().min(60)]);
            }
            folio_cosmic::content::ContentBlock::Image { src, alt } => {
                eprintln!("  [{}] IMG: src={:?} alt={:?}", i, src, alt);
            }
            folio_cosmic::content::ContentBlock::Separator => {
                eprintln!("  [{}] SEP", i);
            }
        }
    }
}
