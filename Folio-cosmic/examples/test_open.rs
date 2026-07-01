use folio_cosmic::document::DocumentModel;
use std::path::PathBuf;
use std::time::Instant;

fn main() {
    let paths = vec![
        "../Folio/output.epub",
        "/home/pedro/Documentos/Guten/Folio/output.epub",
    ];

    let path = paths
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .unwrap_or_else(|| {
            eprintln!("No EPUB found");
            std::process::exit(1);
        });

    eprintln!("Opening {:?}...", path);
    let start = Instant::now();
    match DocumentModel::open(&path) {
        Ok(mut doc) => {
            let elapsed = start.elapsed();
            eprintln!("Opened in {:.2}s", elapsed.as_secs_f64());
            eprintln!("Spine len: {}", doc.spine_len());
            let first = doc.find_first_content_chapter();
            eprintln!("First content: spine[{}]", first);
            doc.goto_spine_index(first);
            match doc.current_chapter_html() {
                Ok(html) => eprintln!("Chapter: {} bytes", html.len()),
                Err(e) => eprintln!("Chapter error: {}", e),
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
