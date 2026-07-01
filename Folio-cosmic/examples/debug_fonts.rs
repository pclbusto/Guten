use folio_cosmic::document::DocumentModel;
use folio_cosmic::fonts::extract_epub_fonts;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("/home/pedro/Descargas/Ready Player Two - Ernest Cline.epub");
    eprintln!("Opening {:?}...", path);
    let mut doc = match DocumentModel::open(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };
    eprintln!("Opened OK. Manifest items:");
    for (id, item) in &doc.core.manifest {
        eprintln!(
            "  [{}] href={} media_type={}",
            id, item.href, item.media_type
        );
    }

    eprintln!("\nExtracting fonts...");
    let fonts = extract_epub_fonts(&doc.core.manifest, |id| doc.core.get_resource_path(id));
    eprintln!("Found {} fonts:", fonts.len());
    for f in &fonts {
        eprintln!("  family_name={} family={:?}", f.family_name, f.family);
    }
}
