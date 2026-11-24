# SC2K Library Manager

Gestor de librerías para DJ de escritorio, escrito en Rust con egui/eframe.
Abre colecciones de Rekordbox y Traktor en **solo lectura**, las unifica en una
misma vista y permite explorarlas por playlist, buscar pistas y filtrar por
armonía musical.

La aplicación es completamente **offline**: no usa red, no envía telemetría y
nunca modifica la librería original ni los archivos de audio.

> Estado: **alpha 0.1.0**. La importación y la exploración ya funcionan.
> La conversión entre programas y la exportación todavía están pendientes.

## Funcionalidades

- **Detección automática** de la exportación XML de Rekordbox y de las
  colecciones de Traktor instaladas en el equipo.
- **Lectura en solo lectura** de pistas, playlists, carpetas, colores, claves,
  BPM, duración y puntos de cue.
- **Comprobación de procesos**: si Rekordbox o Traktor están abiertos, la
  librería no se abre para evitar lecturas a medio escribir.
- **Buscador** por título, artista y álbum, sin reconstruir la lista en cada
  fotograma.
- **Panel de playlists** con "Toda la librería" y árbol de carpetas.
- **Lista virtualizada** con color, portada, título, artista, BPM, clave Camelot
  y duración. Pensada para colecciones de decenas de miles de pistas.
- **Rejilla con portadas** de tarjetas de 120 px.
- **Panel de detalle** con portada grande, metadatos y número de cues.
- **Portadas reales**:
  - arte incrustado en MP3 (ID3v2 `APIC`/`PIC`);
  - arte en chunks `ID3` de AIFF/WAV;
  - RGBA crudo de Traktor en `Coverart/`.
  - Si no hay portada, se dibuja un sustituto con la inicial y el color de la
    pista.
- **Armonía Camelot** de 1A a 12B, con compatibilidad por misma posición o ±1
  con la misma letra y vuelta de 12 a 1.
- **Interfaz en español**, centralizada para poder añadir otros idiomas.

## Programas compatibles

| Programa | Fuente leída | Estado |
| --- | --- | --- |
| Rekordbox 6/7 | `rekordbox.xml` + arte incrustado en el audio | Lectura |
| Traktor 3 | `collection.nml` + `Coverart/` | Lectura |
| Serato DJ | — | Pendiente |
| Virtual DJ | — | Pendiente |
| Engine DJ | — | Pendiente |

El selector de origen y destino ya muestra los cinco programas, pero solo
Rekordbox y Traktor tienen lector. Los escritores y la conversión están
pendientes: el botón **Convertir** y la opción **Exportar** todavía no generan
archivos.

### Rekordbox

Rekordbox 6 y 7 guardan la colección en `master.db`, una base cifrada que esta
aplicación no intenta leer ni modificar. En su lugar se usa la exportación XML:

1. Abre Rekordbox.
2. Ve a **Preferencias > Avanzado > Base de datos** y exporta la colección a
   `rekordbox.xml`.
3. Cierra Rekordbox antes de abrir la librería en este programa.

La exportación puede no incluir los últimos cambios. Los metadatos vienen del
XML y las portadas del arte incrustado en cada archivo de audio. Si un archivo
no existe o no tiene arte, se muestra el sustituto con la inicial.

### Traktor

Traktor guarda la colección en `collection.nml` dentro de su carpeta de usuario
(en Windows, `Documentos\Native Instruments\Traktor <versión>`). Las portadas
están en `Coverart/<carpeta>/<COVERARTID>000` como RGBA crudo.

Cierra Traktor antes de abrir la librería.

## Requisitos

- **Windows 10/11** para la detección de procesos. En macOS/Linux la aplicación
  puede compilar, pero la comprobación de procesos está pendiente.
- **Rust 1.76 o superior** (edición 2021).
- Rekordbox con la colección exportada a XML, o Traktor con `collection.nml`.

## Compilar y ejecutar

```bash
git clone https://github.com/SergioC2K/sc2k-library-manager.git
cd sc2k-library-manager
cargo run --release
```

La primera compilación descarga las dependencias de Rust. La aplicación en sí no
hace ninguna petición de red.

Para compilar sin ejecutar:

```bash
cargo build --release
```

El binario queda en `target/release/dj-library-manager.exe` (Windows).

## Uso

1. Abre **Archivo > Detectar librerías...** o pulsa `Ctrl+O`.
2. Elige la librería encontrada y pulsa **Abrir**.
3. Selecciona una playlist o **Toda la librería**.
4. Cambia entre **Lista** y **Rejilla** desde la cabecera o el menú **Ver**.
5. Selecciona una pista para ver su detalle y su portada.
6. Activa **Solo claves compatibles** para filtrar por armonía respecto a la
   pista seleccionada.

### Atajos de teclado

| Atajo | Acción |
| --- | --- |
| `Ctrl+O` | Detectar y abrir una librería |
| `Ctrl+F` | Enfocar el buscador |
| `Flecha arriba` / `Flecha abajo` | Cambiar de pista seleccionada |
| `Esc` | Cerrar el diálogo de detección |

## Arquitectura

```text
src/
  main.rs            Arranque de eframe y ventana principal
  app.rs             Estado, filtros, hilos y composición de paneles
  model.rs           Modelo neutral: Library, Track, PlaylistNode, CoverSource
  harmony.rs         Rueda Camelot, compatibilidad y color de la clave
  detection.rs       Búsqueda de librerías locales y proveedor de lectura
  process_guard.rs   Comprobación de que Rekordbox/Traktor están cerrados
  readers/
    mod.rs           Utilidades comunes y conversión de claves
    rekordbox_xml.rs Lector de rekordbox.xml
    traktor.rs       Lector de collection.nml
  artwork.rs         Extracción de ID3/APIC y chunks de AIFF/RIFF
  covers.rs          Caché de texturas, hilo de decodificación y sustituto
  ui/
    top_bar.rs       Menús, origen/destino, Convertir y buscador
    playlists.rs     Árbol de playlists
    list.rs          Lista virtualizada con TableBuilder
    grid.rs          Rejilla de portadas
    detail.rs        Panel de detalle
    detect.rs        Diálogo de detección
    chips.rs         Chips de clave y puntos de color
    status.rs        Barra de estado
  i18n.rs            Textos en español
  theme.rs           Tema oscuro y colores
  sample.rs          Datos de ejemplo para las pruebas
```

Puntos de diseño:

- La interfaz no conoce los formatos DJ: solo consume el modelo neutral.
- Las operaciones pesadas (detectar, leer, decodificar portadas) corren en hilos
  de fondo y comunican el progreso por `std::sync::mpsc`.
- La lista trabaja con índices de pistas y está virtualizada; no clona la
  colección en cada fotograma.
- Las portadas se decodifican fuera del hilo de dibujo, se limitan a 256 px y se
  cachean como texturas reutilizables.
- Ningún lector abre archivos en modo escritura.

## Pruebas

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Las pruebas manuales leen las librerías reales del equipo y están ignoradas por
defecto:

```bash
cargo test real_libraries_parse -- --ignored --nocapture
cargo test real_covers_decode_end_to_end -- --ignored --nocapture
```

## Privacidad

- Sin red, sin cuentas y sin telemetría.
- Sin escritura en las librerías de Rekordbox o Traktor.
- Sin modificación de archivos de audio.
- Las portadas solo se leen y se muestran; no se copian fuera de la caché en
  memoria.

## Limitaciones y siguiente pasos

- Escritores y conversión para Rekordbox, Serato DJ, Traktor, Virtual DJ y
  Engine DJ.
- Exportación de la vista o de playlists.
- Lectores para Serato, Virtual DJ y Engine DJ.
- Comprobación de procesos en macOS y Linux.
- Caché persistente de portadas entre sesiones.
- Internacionalización: los textos ya están centralizados, falta el inglés.

## Licencia

Este repositorio todavía no incluye un archivo de licencia. Hasta que se añada,
todos los derechos quedan reservados.
