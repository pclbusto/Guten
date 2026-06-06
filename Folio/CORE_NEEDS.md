# Qué le falta al Core (GutenAIR) para un lector 100% funcional

## Estado actual del lector (GutenReader)

El lector ya implementa:

| Feature | Estado | Notas |
|---------|--------|-------|
| Abrir EPUB (archivo y carpeta) | ✅ | `GutenCore::open_epub` / `open_folder` |
| Navegación spine (capítulos) | ✅ | Usa `core.spine` y `get_resource_path` |
| Renderizado HTML/CSS | ✅ | WebKitGTK 6.0 con inyección de CSS propio |
| Modo dos páginas | ✅ | CSS `column-count: 2` inyectado en el body |
| Modo foco / distracción cero | ✅ | Oculta header, bottom bar, TOC |
| Perfiles de iluminación | ✅ | Día / Noche / Sepia con brillo/contraste/calidez |
| Modo oscuro inteligente | ✅ | Invierte texto/fondo sin tocar imágenes (CSS) |
| TTS (text-to-speech) | ⚠️ MVP | Usa `spd-say` o `espeak` vía comando; extrae texto con JS desde WebKit |
| Búsqueda en texto | ⚠️ MVP | Usa índice FTS5 del core (`core.search`); navegación a resultados implementada |
| Posición de lectura persistente | ⚠️ MVP | Guarda `spine_index` al cambiar de capítulo / cerrar app |
| TOC (tabla de contenidos) | ✅ | `core.get_toc()` + panel lateral |
| Anotaciones | ⚠️ Modelo base | `AnnotationStore` persiste JSON, pero falta UI de subrayado visual en WebKit |
| Estadísticas de lectura | ⚠️ Modelo base | `core.get_book_stats()` / `get_chapter_stats()` existen pero no hay panel UI avanzado |
| Personalización CSS completa | ⚠️ MVP | El lector inyecta CSS propio, pero no analiza el CSS original del EPUB |

---

## Falta en el Core para que el lector sea completo

### 1. Extracción de texto plano por capítulo
**Prioridad: Alta**

El lector necesita obtener el **texto plano limpio** de un capítulo para:
- TTS nativo (sin depender de JS del WebView).
- Estadísticas de lectura en tiempo real (palabras leídas, velocidad, tiempo restante).
- Diccionario / traducción instantánea (seleccionar palabra → consultar).

**Qué falta:**
- Un método `GutenCore::get_chapter_plaintext(id: &str) -> Result<String>` que use `ammonia` + limpieza para devolver solo texto legible, preservando párrafos.

### 2. API de posición de lectura granular (CFI o similar)
**Prioridad: Alta**

Actualmente solo guardamos `spine_index`. Para sincronización entre dispositivos y estadísticas reales, necesitamos:
- Offset de carácter o porcentaje dentro del capítulo.
- Navegación precisa a un punto dentro del capítulo (no solo al inicio).

**Qué falta:**
- `ReadingPosition { spine_index, char_offset: usize, percent: f64 }`
- Método para calcular `char_offset` a partir de `block_id` y viceversa (necesita el índice del core).

### 3. Anotaciones / highlights integradas al EPUB
**Prioridad: Media**

El lector guarda anotaciones en JSON aparte. Esto es frágil: si el EPUB cambia, los offsets se rompen.

**Qué falta:**
- Un modelo de anotaciones en el core basado en CFI (EPUB Canonical Fragment Identifier) o en `block_id + text_offset`.
- Persistencia dentro del EPUB o en un archivo `.gutenair.annotations.json` estandarizado.
- Método `core.add_annotation(chapter_id, block_id, start, end, note, color)` y `core.get_annotations(chapter_id)`.

### 4. Resolución de CSS aplicado a un capítulo
**Prioridad: Media**

El lector permite personalizar CSS, pero no sabe qué CSS original aplica el EPUB a cada capítulo. Para respetar o anular estilos de forma inteligente, necesitamos saber qué hojas de estilo están vinculadas.

**Qué falta:**
- `core.get_linked_stylesheets(chapter_id) -> Vec<String>`: devuelve las rutas de los `<link rel="stylesheet">` de un capítulo XHTML.
- `core.get_combined_css(chapter_id) -> Result<String>`: opcionalmente lee y concatena los CSS vinculados.

### 5. Corrección de errores comunes de EPUB
**Prioridad: Media**

El core ya tiene `clean_html` y `sanitize_to_xhtml`, pero esos son para **escritura**. Para un lector, necesitamos corrección automática de EPUBs rotos al **leer**:
- Rutas relativas mal formadas en imágenes/CSS.
- Etiquetas HTML5 void elements mal cerradas (roxmltree falla).
- `DOCTYPE` que rompe el parser (ya se strippea en algunos lugares, pero no globalmente).
- Links internos rotos: el core ya tiene `validate_links`, pero no un auto-fix.

**Qué falta:**
- `core.get_chapter_html_safe(id) -> Result<String>`: lee el capítulo, aplica fixes automáticos (void elements, strip DOCTYPE, rutas relativas corregidas), y devuelve HTML válido para renderizar.

### 6. Índice de palabras para diccionario
**Prioridad: Baja**

Para "tocar una palabra y ver significado", el lector necesita saber la palabra bajo el cursor. WebKit puede dar eso con JS, pero para diccionarios offline (Stardict, WordNet), el core podría ayudar:

**Qué falta:**
- No estrictamente necesario en el core, pero un `gutencore::index::WordIndex` separado sería ideal.

### 7. Soporte de SVG como recurso vectorial
**Prioridad: Baja**

WebKit ya renderiza SVG. Pero para zoom sin pérdida de calidad, el lector necesita que el core:
- Identifique recursos SVG en el manifiesto.
- Permita extraer el SVG como string para renderizado directo o conversión.

El core ya puede leer cualquier recurso con `get_resource_path`, así que esto es más bien responsabilidad del lector.

---

## Falta en el Lector (UI / lógica propia)

Algunas features solicitadas son puramente UI y no requieren cambios en el core, pero aún no están implementadas:

| Feature | Estado | Por qué falta |
|---------|--------|---------------|
| Anotaciones visuales (subrayado en WebKit) | ❌ | Requiere JS injection + rangos de selección + persistencia CFI |
| Estadísticas de lectura UI (tiempo restante, velocidad) | ❌ | Requiere timer de lectura + texto plano + posición granular |
| Sincronización entre dispositivos | ❌ | Requiere backend WebDAV/Nextcloud |
| Diccionario externo / traducción | ❌ | Requiere integración con APIs o diccionarios locales |
| Corrección de EPUBs sobre la marcha | ❌ | Requiere `safe_html` en el core + lógica de fallback en el lector |
| Soporte avanzado HTML5/CSS3 | ⚠️ Parcial | WebKit 6.0 soporta mucho, pero EPUBs mal formados pueden fallar |

---

## Recomendaciones de arquitectura

1. **Core → Lector**:
   - Delegar TODO lo relacionado con parseo, metadatos, manifiesto, spine, TOC, búsqueda FTS5, estadísticas base y validación al core.
   - No duplicar lógica de resolución de rutas ni parseo XML en el lector.

2. **Lector → Core**:
   - El lector debe pedirle al core el HTML ya corregido (`get_chapter_html_safe`).
   - El lector debe pedirle al core el texto plano (`get_chapter_plaintext`).
   - El lector debe usar el modelo de anotaciones del core en cuanto esté disponible.

3. **Mejora inmediata del core**:
   - Implementar `get_chapter_plaintext(id)`.
   - Implementar `get_chapter_html_safe(id)` con auto-fix de errores comunes.
   - Exponer `get_linked_stylesheets(id)`.
   - Agregar modelo de `Annotation` / `Highlight` al `GutenConfig` o archivo separado.

---

## Conclusión

El lector es funcional para lectura básica con EPUBs bien formados. Las features avanzadas (anotaciones enriquecidas, estadísticas detalladas, sincronización, diccionario) requieren **extender el core** para que exponga texto plano, posiciones granulares, y un modelo de anotaciones robusto. Sin eso, el lector se ve forzado a reimplementar lógica de parseo HTML o a depender de hacks de WebKit/JS.
