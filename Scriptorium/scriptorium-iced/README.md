# Scriptorium COSMIC

Cliente de escritorio en Rust con `libcosmic` para la biblioteca compartida de
Scriptorium.

## Estado

Primer corte funcional:

- biblioteca en cuadrícula y lista;
- búsqueda FTS por título o autor;
- importación múltiple de EPUB;
- ficha y edición de título/autor;
- apertura con el lector predeterminado;
- estadísticas globales;
- uso de la misma base SQLite que el CLI y `scriptorium-gtk`.

## Ejecutar

```bash
cargo run
```

La base de datos se encuentra en `$XDG_DATA_HOME/rubrica/library.db` o, por
defecto, `~/.local/share/rubrica/library.db`.

## UI

La aplicación usa `libcosmic`, no Iced puro. La biblioteca muestra todos los
libros con el diseño de tarjeta seleccionado. Las doce variantes y su selector
están en Configuración; la preferencia se conserva entre ejecuciones.

Los diseños implementan el contrato `BookCardDesign`. Biblioteca y la vista
previa llaman al mismo método `render`, por lo que no pueden divergir. Para
incorporar otra tarjeta se crea un tipo que implemente ese trait y se agrega una
referencia a su instancia en `CARD_DESIGNS`. El registro acepta implementaciones
heterogéneas, no solamente las variantes incluidas.

Las portadas extraídas se guardan en `$XDG_CACHE_HOME/scriptorium/covers` (o
`~/.cache/scriptorium/covers`). La clave deriva del hash persistido del EPUB:
solo los libros nuevos o modificados necesitan volver a abrirse. Durante una
ejecución, los handles de imagen también se conservan en memoria. La cuadrícula
usa miniaturas limitadas a 360×540 px para evitar decodificar y subir a la GPU
las portadas originales de gran tamaño, y pagina el catálogo en grupos
configurables (12, 24, 48, 72 o 96 carátulas) para mantener acotada la cantidad
de tarjetas activas.
