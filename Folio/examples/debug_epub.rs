use gutencore::GutenCore;
use std::env;

fn main() {
    let path = env::args().nth(1).expect("Uso: debug_epub <ruta al .epub>");

    let mut core = GutenCore::open_epub(&path).expect("No se pudo abrir el EPUB");

    println!("=== METADATA ===");
    if let Some(m) = &core.metadata {
        println!("Título: {}", m.title);
        println!("Autor: {:?}", m.author);
        println!("Idioma: {}", m.language);
        println!("Identificador: {}", m.identifier);
    }

    println!("\n=== SPINE ({} items) ===", core.spine.len());
    for (i, id) in core.spine.iter().enumerate() {
        if let Ok(item) = core.get_item(id) {
            println!(
                "  [{}] id={} href={} media_type={}",
                i, id, item.href, item.media_type
            );
        } else {
            println!("  [{}] id={} (NO EN MANIFIESTO)", i, id);
        }
    }

    println!("\n=== PRIMER CAPÍTULO ===");
    if let Some(first_id) = core.spine.first() {
        if let Ok(item) = core.get_item(first_id) {
            let path = core.get_resource_path(first_id).unwrap();
            let html = std::fs::read_to_string(&path).unwrap();
            println!(
                "id={} href={}\n--- HTML (primeros 800 chars) ---",
                first_id, item.href
            );
            println!("{}", &html[..html.len().min(800)]);
        }
    }

    println!("\n=== ÍNDICE DE BÚSQUEDA ===");
    let results = core.search("el").unwrap();
    println!("Búsqueda de 'el': {} resultados", results.len());
    for r in results.iter().take(5) {
        println!(
            "  chap={} block={} snippet={}",
            r.chapter_id, r.block_id, r.snippet
        );
    }
}
