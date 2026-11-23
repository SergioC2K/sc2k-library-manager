//! Lectores de librerías locales en solo lectura.
//!
//! - Rekordbox 6 se lee desde su exportación XML (`rekordbox.xml`); la base
//!   `master.db` está cifrada y no se toca.
//! - Traktor se lee desde `collection.nml`.
//!
//! Ningún lector escribe en los archivos del usuario.

pub mod rekordbox_xml;
pub mod traktor;

use quick_xml::events::BytesStart;

use crate::harmony::{CamelotKey, KeyMode};
use crate::model::TrackColor;

/// Atributo XML sin escapar, o `None` si no existe o no es UTF-8 válido.
pub(crate) fn attr(element: &BytesStart<'_>, name: &[u8]) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == name)
        .and_then(|attribute| attribute.unescape_value().ok())
        .map(|value| value.into_owned())
}

/// Convierte a Camelot las notaciones que aparecen en las librerías:
/// - Open Key (`1d`..`12d`, `1m`..`12m`), usada por Traktor.
/// - Camelot ya resuelta (`8A`, `12b`).
/// - Nombres musicales (`C`, `F#m`, `Ab`, `Cmin`, `Bbm`...), usados por
///   Rekordbox y por algunas pistas de Traktor.
///
/// En Open Key, `1d` es Do mayor (8B) y `1m` es La menor (8A).
pub(crate) fn camelot_from_key_text(text: &str) -> Option<CamelotKey> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(key) = CamelotKey::parse(text) {
        return Some(key);
    }
    if let Some(key) = camelot_from_open_key(text) {
        return Some(key);
    }
    camelot_from_musical_key(text)
}

fn camelot_from_open_key(text: &str) -> Option<CamelotKey> {
    let mut chars = text.chars();
    let suffix = chars.next_back()?.to_ascii_lowercase();
    let digits = chars.as_str().trim();
    let number: u8 = digits.parse().ok()?;
    if !(1..=12).contains(&number) {
        return None;
    }

    let mode = match suffix {
        'd' => KeyMode::Major,
        'm' => KeyMode::Minor,
        _ => return None,
    };
    // 1 -> 8, 2 -> 9, ..., 6 -> 1, ..., 12 -> 7.
    let camelot = ((number + 6) % 12) + 1;
    CamelotKey::new(camelot, mode)
}

fn camelot_from_musical_key(text: &str) -> Option<CamelotKey> {
    let normalized: String = text
        .trim()
        .chars()
        .map(|character| match character {
            '♯' => '#',
            '♭' => 'b',
            other => other,
        })
        .collect();
    let normalized = normalized.trim().to_ascii_lowercase();

    let (root, minor) = if let Some(root) = normalized.strip_suffix("min") {
        (root, true)
    } else if let Some(root) = normalized.strip_suffix("maj") {
        (root, false)
    } else if let Some(root) = normalized.strip_suffix('m') {
        (root, true)
    } else {
        (normalized.as_str(), false)
    };
    let root = root.trim();
    if root.is_empty() {
        return None;
    }

    let (table, mode) = if minor {
        (MINOR_KEYS, KeyMode::Minor)
    } else {
        (MAJOR_KEYS, KeyMode::Major)
    };
    let number = table
        .iter()
        .find(|(name, _)| *name == root)
        .map(|(_, number)| *number)?;
    CamelotKey::new(number, mode)
}

/// Raíz mayor -> número Camelot.
const MAJOR_KEYS: &[(&str, u8)] = &[
    ("c", 8),
    ("g", 9),
    ("d", 10),
    ("a", 11),
    ("e", 12),
    ("b", 1),
    ("cb", 1),
    ("f#", 2),
    ("gb", 2),
    ("db", 3),
    ("c#", 3),
    ("ab", 4),
    ("g#", 4),
    ("eb", 5),
    ("d#", 5),
    ("bb", 6),
    ("a#", 6),
    ("f", 7),
];

/// Raíz menor -> número Camelot.
const MINOR_KEYS: &[(&str, u8)] = &[
    ("a", 8),
    ("e", 9),
    ("b", 10),
    ("cb", 10),
    ("f#", 11),
    ("gb", 11),
    ("c#", 12),
    ("db", 12),
    ("g#", 1),
    ("ab", 1),
    ("d#", 2),
    ("eb", 2),
    ("a#", 3),
    ("bb", 3),
    ("f", 4),
    ("c", 5),
    ("g", 6),
    ("d", 7),
];

/// Ruta local a partir del `Location` de Rekordbox
/// (`file://localhost/C:/Musica/tema.mp3`, con escapes como `%20`).
pub(crate) fn file_url_to_path(text: &str) -> Option<std::path::PathBuf> {
    let rest = text
        .trim()
        .strip_prefix("file://localhost/")
        .or_else(|| text.trim().strip_prefix("file:///"))?;
    let decoded = percent_decode(rest)?;
    if decoded.is_empty() {
        return None;
    }
    #[cfg(windows)]
    let decoded = decoded.replace('/', "\\");
    Some(std::path::PathBuf::from(decoded))
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            let value = (hex_digit(high)? << 4) | hex_digit(low)?;
            out.push(value);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Segundos desde un atributo entero (o decimal) del XML.
pub(crate) fn parse_seconds(text: &str) -> u64 {
    let text = text.trim();
    text.parse::<u64>()
        .or_else(|_| {
            text.parse::<f64>()
                .map(|value| value.max(0.0).round() as u64)
        })
        .unwrap_or(0)
}

/// BPM finito y no negativo; 0.0 si el valor no sirve.
pub(crate) fn parse_bpm(text: &str) -> f32 {
    text.trim()
        .parse::<f32>()
        .ok()
        .filter(|bpm| bpm.is_finite() && *bpm >= 0.0)
        .unwrap_or(0.0)
}

/// Color de pista de Rekordbox (`0xRRGGBB`) al color neutral más cercano.
pub(crate) fn track_color_from_hex(text: &str) -> TrackColor {
    let text = text.trim();
    let digits = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))
        .unwrap_or(text);
    let Ok(mut value) = u32::from_str_radix(digits, 16) else {
        return TrackColor::None;
    };
    if digits.len() > 6 {
        // Algunas exportaciones usan ARGB; el color útil son los 24 bits bajos.
        value &= 0x00FF_FFFF;
    }
    if value == 0 {
        return TrackColor::None;
    }

    let rgb = (
        ((value >> 16) & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        (value & 0xFF) as u8,
    );
    nearest_track_color(rgb)
}

/// Color de pista de Traktor: identificador 1..=7 de su paleta.
pub(crate) fn track_color_from_traktor_id(text: &str) -> TrackColor {
    match text.trim().parse::<u8>() {
        Ok(1) => TrackColor::Red,
        Ok(2) => TrackColor::Orange,
        Ok(3) => TrackColor::Yellow,
        Ok(4) => TrackColor::Green,
        Ok(5) => TrackColor::Blue,
        Ok(6) => TrackColor::Purple,
        Ok(7) => TrackColor::Pink,
        _ => TrackColor::None,
    }
}

fn nearest_track_color(rgb: (u8, u8, u8)) -> TrackColor {
    const PALETTE: [(TrackColor, (u8, u8, u8)); 7] = [
        (TrackColor::Pink, (240, 120, 190)),
        (TrackColor::Red, (226, 76, 76)),
        (TrackColor::Orange, (236, 148, 62)),
        (TrackColor::Yellow, (226, 202, 74)),
        (TrackColor::Green, (96, 190, 112)),
        (TrackColor::Blue, (84, 150, 230)),
        (TrackColor::Purple, (158, 122, 224)),
    ];

    let mut best = (TrackColor::None, u32::MAX);
    for (color, reference) in PALETTE {
        let distance = squared_distance(rgb, reference);
        if distance < best.1 {
            best = (color, distance);
        }
    }

    // Umbral amplio para aceptar la paleta pura de Rekordbox, pero rechazar
    // blanco/negro (que no son colores de pista válidos).
    if best.1 <= 180 * 180 {
        best.0
    } else {
        TrackColor::None
    }
}

fn squared_distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
    let dr = i32::from(a.0) - i32::from(b.0);
    let dg = i32::from(a.1) - i32::from(b.1);
    let db = i32::from(a.2) - i32::from(b.2);
    (dr * dr + dg * dg + db * db) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CoverSource;

    fn key(number: u8, mode: KeyMode) -> CamelotKey {
        CamelotKey::new(number, mode).expect("clave válida")
    }

    #[test]
    fn open_key_notation_maps_to_camelot() {
        assert_eq!(camelot_from_key_text("1d"), Some(key(8, KeyMode::Major)));
        assert_eq!(camelot_from_key_text("1m"), Some(key(8, KeyMode::Minor)));
        assert_eq!(camelot_from_key_text("11d"), Some(key(6, KeyMode::Major)));
        assert_eq!(camelot_from_key_text("11m"), Some(key(6, KeyMode::Minor)));
        assert_eq!(camelot_from_key_text("10d"), Some(key(5, KeyMode::Major)));
        assert_eq!(camelot_from_key_text("12m"), Some(key(7, KeyMode::Minor)));
        assert_eq!(camelot_from_key_text("8A"), Some(key(8, KeyMode::Minor)));
        assert_eq!(
            camelot_from_key_text(" 12b "),
            Some(key(12, KeyMode::Major))
        );
    }

    #[test]
    fn musical_key_names_map_to_camelot() {
        assert_eq!(camelot_from_key_text("Gm"), Some(key(6, KeyMode::Minor)));
        assert_eq!(camelot_from_key_text("F#m"), Some(key(11, KeyMode::Minor)));
        assert_eq!(camelot_from_key_text("Abm"), Some(key(1, KeyMode::Minor)));
        assert_eq!(camelot_from_key_text("Bbm"), Some(key(3, KeyMode::Minor)));
        assert_eq!(camelot_from_key_text("Cmin"), Some(key(5, KeyMode::Minor)));
        assert_eq!(camelot_from_key_text("C"), Some(key(8, KeyMode::Major)));
        assert_eq!(camelot_from_key_text("Ab"), Some(key(4, KeyMode::Major)));
        assert_eq!(camelot_from_key_text("F#"), Some(key(2, KeyMode::Major)));
        assert_eq!(camelot_from_key_text("Db"), Some(key(3, KeyMode::Major)));
        assert_eq!(camelot_from_key_text("B"), Some(key(1, KeyMode::Major)));
        assert_eq!(camelot_from_key_text("F♯m"), Some(key(11, KeyMode::Minor)));
    }

    #[test]
    fn invalid_keys_are_ignored() {
        assert!(camelot_from_key_text("").is_none());
        assert!(camelot_from_key_text("13d").is_none());
        assert!(camelot_from_key_text("0m").is_none());
        assert!(camelot_from_key_text("8x").is_none());
        assert!(camelot_from_key_text("off").is_none());
    }

    #[test]
    fn rekordbox_colors_map_to_the_neutral_palette() {
        assert_eq!(track_color_from_hex("0xFFFF00"), TrackColor::Yellow);
        assert_eq!(track_color_from_hex("0x00FF00"), TrackColor::Green);
        assert_eq!(track_color_from_hex("0xFF0000"), TrackColor::Red);
        assert_eq!(track_color_from_hex("0x0000FF"), TrackColor::Blue);
        assert_eq!(track_color_from_hex("0x000000"), TrackColor::None);
        assert_eq!(track_color_from_hex(""), TrackColor::None);
        assert_eq!(track_color_from_hex("no-es-color"), TrackColor::None);
    }

    #[test]
    fn traktor_color_ids_are_mapped() {
        assert_eq!(track_color_from_traktor_id("1"), TrackColor::Red);
        assert_eq!(track_color_from_traktor_id("4"), TrackColor::Green);
        assert_eq!(track_color_from_traktor_id("6"), TrackColor::Purple);
        assert_eq!(track_color_from_traktor_id("0"), TrackColor::None);
        assert_eq!(track_color_from_traktor_id("99"), TrackColor::None);
    }

    #[test]
    fn rekordbox_locations_become_local_paths() {
        let path = file_url_to_path("file://localhost/C:/Musica/Mi%20tema%20(%20mix).mp3")
            .expect("ruta válida");
        let text = path.to_string_lossy();
        assert!(text.starts_with("C:"));
        assert!(text.ends_with("Mi tema ( mix).mp3"));
        assert!(file_url_to_path("file://localhost/").is_none());
        assert!(file_url_to_path("C:/sin-esquema.mp3").is_none());
        assert!(file_url_to_path("file://localhost/C:/mal%2.mp3").is_none());
    }

    #[test]
    fn numeric_parsing_is_defensive() {
        assert_eq!(parse_seconds("377"), 377);
        assert_eq!(parse_seconds("131.81"), 132);
        assert_eq!(parse_seconds("-4"), 0);
        assert_eq!(parse_seconds("x"), 0);
        assert_eq!(parse_bpm("128.00"), 128.0);
        assert_eq!(parse_bpm("-1"), 0.0);
        assert_eq!(parse_bpm("NaN"), 0.0);
    }

    /// Comprobación manual contra las librerías reales del equipo.
    /// Se salta sola si no encuentra los archivos habituales.
    #[test]
    #[ignore = "lee las librerías locales del equipo; ejecutar a mano"]
    fn real_libraries_parse() {
        use std::path::PathBuf;

        let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from);

        if let Some(home) = &home {
            let rekordbox = appdata
                .map(|dir| dir.join("Pioneer").join("rekordbox").join("rekordbox.xml"))
                .filter(|path| path.is_file());
            if let Some(path) = rekordbox {
                let library = super::rekordbox_xml::read(&path, &mut |_| {}).expect("rekordbox");
                println!(
                    "Rekordbox: {} pistas, {} playlists",
                    library.tracks.len(),
                    library.playlist_count()
                );
                println!(
                    "  claves {}, colores {}, cues {}",
                    library
                        .tracks
                        .iter()
                        .filter(|track| track.key.is_some())
                        .count(),
                    library
                        .tracks
                        .iter()
                        .filter(|track| track.color != crate::model::TrackColor::None)
                        .count(),
                    library
                        .tracks
                        .iter()
                        .map(|track| usize::from(track.cue_points))
                        .sum::<usize>()
                );
                assert!(!library.tracks.is_empty());
                assert!(library.tracks.iter().any(|track| track.key.is_some()));
                let with_cover = library
                    .tracks
                    .iter()
                    .filter(|track| track.cover.is_some())
                    .count();
                println!("  portadas referenciadas {with_cover}");
                let decoded = library
                    .tracks
                    .iter()
                    .filter_map(|track| track.cover.as_ref())
                    .take(400)
                    .filter(|cover| cover_bytes_decode(cover))
                    .count();
                println!("  portadas decodificadas en la muestra {decoded}/400");
                assert!(decoded > 0, "ninguna portada de Rekordbox se pudo leer");
                assert!(library.playlist_count() > 0);
            }

            let native = home.join("Documents").join("Native Instruments");
            if let Ok(entries) = std::fs::read_dir(&native) {
                for entry in entries.flatten() {
                    let path = entry.path().join("collection.nml");
                    if !path.is_file() {
                        continue;
                    }
                    let library = super::traktor::read(&path, &mut |_| {}).expect("traktor");
                    println!(
                        "Traktor ({}): {} pistas, {} playlists",
                        path.display(),
                        library.tracks.len(),
                        library.playlist_count()
                    );
                    let covers: Vec<_> = library
                        .tracks
                        .iter()
                        .filter_map(|track| track.cover.as_ref())
                        .collect();
                    let decoded = covers
                        .iter()
                        .take(20)
                        .filter(|cover| cover_bytes_decode(cover))
                        .count();
                    println!(
                        "  claves {}, colores {}, cues {}, portadas {} (muestra {decoded}/20)",
                        library
                            .tracks
                            .iter()
                            .filter(|track| track.key.is_some())
                            .count(),
                        library
                            .tracks
                            .iter()
                            .filter(|track| track.color != crate::model::TrackColor::None)
                            .count(),
                        library
                            .tracks
                            .iter()
                            .map(|track| usize::from(track.cue_points))
                            .sum::<usize>(),
                        covers.len()
                    );
                    assert!(!library.tracks.is_empty());
                    assert!(library.tracks.iter().any(|track| track.key.is_some()));
                    assert!(decoded > 0, "ninguna portada de Traktor se pudo leer");
                }
            }
        }
    }

    fn cover_bytes_decode(cover: &CoverSource) -> bool {
        match cover {
            CoverSource::Embedded(path) => crate::artwork::embedded_image(path).is_some(),
            CoverSource::RawRgba(path) => std::fs::read(path)
                .ok()
                .and_then(|bytes| crate::artwork::raw_rgba(&bytes))
                .is_some(),
        }
    }
}
