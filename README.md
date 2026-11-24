# SC2K Library Manager

Gestor de librerías DJ de escritorio, offline y en solo lectura. Reúne
colecciones de Rekordbox y Traktor en una vista común para buscar, explorar
playlists, ver portadas y filtrar por armonía Camelot.

## Estado

Versión 0.1.0. Importación y exploración disponibles. Conversión y exportación
en desarrollo.

## Funciones

- Detección y lectura de Rekordbox (`rekordbox.xml`) y Traktor (`collection.nml`).
- Búsqueda por título, artista y álbum.
- Playlists y carpetas.
- Lista virtualizada y rejilla con portadas.
- Panel de detalle con metadatos y puntos de cue.
- Portadas reales con sustituto por inicial.
- Claves Camelot y filtro por armonía.
- Solo lectura y sin conexión.

## Requisitos

- Windows 10/11.
- Rust 1.76 o superior.
- Rekordbox con la colección exportada a XML, o Traktor con su colección local.

## Inicio rápido

```bash
git clone https://github.com/SergioC2K/sc2k-library-manager.git
cd sc2k-library-manager
cargo run --release
```

## Uso

1. Pulsa `Ctrl+O` o abre **Archivo > Detectar librerías...**.
2. Elige la librería y pulsa **Abrir**.
3. Selecciona una playlist o **Toda la librería**.
4. Cambia entre **Lista** y **Rejilla**.
5. Selecciona una pista para ver su detalle.
6. Activa **Solo claves compatibles** para filtrar por armonía.

Cierra Rekordbox o Traktor antes de abrir su librería.

## Programas

| Programa | Lectura |
| --- | --- |
| Rekordbox | Sí (`rekordbox.xml`) |
| Traktor | Sí (`collection.nml`) |
| Serato DJ, Virtual DJ, Engine DJ | Pendiente |

## Privacidad

La aplicación no usa red, no envía datos y nunca modifica las librerías
originales ni los archivos de audio.

## Licencia

Sin licencia definida por ahora. Todos los derechos reservados.
