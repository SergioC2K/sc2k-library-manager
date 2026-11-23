//! Extracción de portadas desde archivos locales, en solo lectura.
//!
//! - Traktor guarda cada portada en `Coverart/` como RGBA sin comprimir con una
//!   cabecera de 9 bytes: versión, ancho y alto en `u32` little-endian.
//! - Rekordbox 6 no expone la ruta de la portada en su XML, así que se lee la
//!   imagen incrustada en el archivo de audio (etiqueta ID3v2 `APIC`, también en
//!   el chunk `ID3 ` de AIFF/WAV).
//!
//! Estas funciones no dibujan ni tocan la GPU: se llaman desde el hilo de
//! portadas. Si no hay imagen válida devuelven `None` y la UI usa el sustituto.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Tamaño máximo de etiqueta que se acepta, para no leer basura gigante.
const MAX_TAG: u64 = 32 * 1024 * 1024;

/// Portada RGBA ya desempaquetada del formato de Traktor.
pub struct RawImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Decodifica el formato de portada de Traktor.
///
/// Cabecera: `[versión = 8][ancho u32 LE][alto u32 LE]` y después
/// `ancho * alto * 4` bytes RGBA. Se rechaza cualquier archivo truncado.
pub fn raw_rgba(bytes: &[u8]) -> Option<RawImage> {
    if bytes.len() < 9 || bytes[0] != 8 {
        return None;
    }
    let width = u32::from_le_bytes(bytes[1..5].try_into().ok()?);
    let height = u32::from_le_bytes(bytes[5..9].try_into().ok()?);
    if width == 0 || height == 0 {
        return None;
    }
    let pixels = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?
        .checked_mul(4)?;
    if bytes.len() < 9 + pixels {
        return None;
    }
    Some(RawImage {
        width,
        height,
        pixels: bytes[9..9 + pixels].to_vec(),
    })
}

/// Bytes de la imagen (JPEG/PNG) incrustada en un archivo de audio.
///
/// Admite MP3 con etiqueta ID3v2 al principio, AIFF y WAV con su chunk `ID3 `.
/// Devuelve `None` si el archivo no se puede abrir o no tiene imagen.
pub fn embedded_image(path: &Path) -> Option<Vec<u8>> {
    let tag = id3_tag(path)?;
    id3_picture(&tag)
}

/// Localiza y lee la etiqueta ID3v2 del archivo, sin leer el audio entero.
fn id3_tag(path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let mut magic = [0u8; 12];
    file.read_exact(&mut magic).ok()?;

    match &magic[0..4] {
        b"FORM" if matches!(&magic[8..12], b"AIFF" | b"AIFC") => {
            chunked_tag(&mut file, ChunkOrder::Big, &[b"ID3 "])
        }
        b"RIFF" if &magic[8..12] == b"WAVE" => {
            chunked_tag(&mut file, ChunkOrder::Little, &[b"id3 ", b"ID3 "])
        }
        _ if &magic[0..3] == b"ID3" => {
            let size = syncsafe(&magic[6..10])? as u64;
            let body_len = usize::try_from(size.min(MAX_TAG)).ok()?;
            let mut tag = vec![0u8; 10 + body_len];
            tag[..10].copy_from_slice(&magic[..10]);
            file.read_exact(&mut tag[10..]).ok()?;
            Some(tag)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum ChunkOrder {
    Big,
    Little,
}

/// Recorre los chunks de un AIFF/RIFF hasta encontrar el de ID3.
fn chunked_tag(file: &mut File, order: ChunkOrder, wanted: &[&[u8; 4]]) -> Option<Vec<u8>> {
    let length = file.metadata().ok()?.len();
    let mut position = 12u64;

    while position + 8 <= length {
        file.seek(SeekFrom::Start(position)).ok()?;
        let mut header = [0u8; 8];
        file.read_exact(&mut header).ok()?;
        let size = match order {
            ChunkOrder::Big => u32::from_be_bytes(header[4..8].try_into().ok()?),
            ChunkOrder::Little => u32::from_le_bytes(header[4..8].try_into().ok()?),
        } as u64;

        if wanted.iter().any(|id| header[0..4] == id[..]) {
            let read = usize::try_from(size.min(MAX_TAG)).ok()?;
            let mut tag = vec![0u8; read];
            file.read_exact(&mut tag).ok()?;
            return Some(tag);
        }
        // Los chunks van alineados a dos bytes.
        position += 8 + size + (size & 1);
    }
    None
}

/// Entero de 28 bits usado por ID3v2 para tamaños (7 bits por byte).
fn syncsafe(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 4 || bytes.iter().any(|byte| byte & 0x80 != 0) {
        return None;
    }
    Some(
        (u32::from(bytes[0]) << 21)
            | (u32::from(bytes[1]) << 14)
            | (u32::from(bytes[2]) << 7)
            | u32::from(bytes[3]),
    )
}

/// Devuelve los bytes de imagen del primer `APIC` (o `PIC` en ID3v2.2).
fn id3_picture(tag: &[u8]) -> Option<Vec<u8>> {
    if tag.len() < 10 || &tag[0..3] != b"ID3" {
        return None;
    }
    let version = tag[3];
    if !(2..=4).contains(&version) {
        return None;
    }
    let flags = tag[5];
    let size = syncsafe(&tag[6..10])? as usize;
    let mut body = &tag[10..(10 + size).min(tag.len())];

    if flags & 0x40 != 0 {
        body = skip_extended_header(body, version)?;
    }

    let id_len = if version == 2 { 3 } else { 4 };
    let header_len = if version == 2 { 6 } else { 10 };
    let mut offset = 0usize;
    while offset + header_len <= body.len() {
        let id = &body[offset..offset + id_len];
        if id[0] == 0 {
            break;
        }
        let (size, flags) = match version {
            2 => (
                u32::from_be_bytes([0, body[offset + 3], body[offset + 4], body[offset + 5]])
                    as usize,
                None,
            ),
            3 => (
                u32::from_be_bytes(body[offset + 4..offset + 8].try_into().ok()?) as usize,
                Some(&body[offset + 8..offset + 10]),
            ),
            _ => (
                syncsafe(&body[offset + 4..offset + 8])? as usize,
                Some(&body[offset + 8..offset + 10]),
            ),
        };

        let start = offset + header_len;
        let end = start.checked_add(size)?;
        if end > body.len() {
            break;
        }
        let is_picture = if version == 2 {
            id == b"PIC"
        } else {
            id == b"APIC"
        };
        if is_picture && !frame_is_encoded(flags, version) {
            return picture_data(&body[start..end], version);
        }
        offset = end;
    }
    None
}

fn skip_extended_header(body: &[u8], version: u8) -> Option<&[u8]> {
    if body.len() < 4 {
        return None;
    }
    let extra = match version {
        // En ID3v2.3 el tamaño no se cuenta a sí mismo.
        3 => u32::from_be_bytes(body[0..4].try_into().ok()?) as usize + 4,
        // En ID3v2.4 el tamaño incluye la propia cabecera.
        4 => syncsafe(&body[0..4])? as usize,
        _ => return None,
    };
    body.get(extra..)
}

/// Marcos comprimidos o cifrados no se pueden interpretar sin más contexto.
fn frame_is_encoded(flags: Option<&[u8]>, version: u8) -> bool {
    let Some(flags) = flags else {
        return false;
    };
    let Some(&format) = flags.get(1) else {
        return false;
    };
    if version == 4 {
        format & 0x0C != 0
    } else {
        format & 0xC0 != 0
    }
}

/// Separa la imagen del resto del marco `APIC`/`PIC`.
fn picture_data(frame: &[u8], version: u8) -> Option<Vec<u8>> {
    let encoding = *frame.first()?;
    if version == 2 {
        if frame.len() < 5 {
            return None;
        }
        let rest = &frame[4..];
        let start = description_end(rest, encoding)?;
        return Some(rest[start..].to_vec());
    }

    let mime_end = frame.iter().skip(1).position(|&byte| byte == 0)? + 1;
    let rest = frame.get(mime_end + 1..)?;
    // Tipo de imagen (1 byte) y después la descripción.
    let rest = rest.get(1..)?;
    let start = description_end(rest, encoding)?;
    Some(rest[start..].to_vec())
}

/// Índice justo después del terminador de la descripción.
fn description_end(bytes: &[u8], encoding: u8) -> Option<usize> {
    if encoding == 1 || encoding == 2 {
        let mut index = 0;
        while index + 1 < bytes.len() {
            if bytes[index] == 0 && bytes[index + 1] == 0 {
                return Some(index + 2);
            }
            index += 2;
        }
        None
    } else {
        bytes
            .iter()
            .position(|&byte| byte == 0)
            .map(|index| index + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const PICTURE: &[u8] = b"\xFF\xD8\xFF\xE0imagen-de-prueba";

    fn syncsafe_bytes(value: u32) -> [u8; 4] {
        [
            ((value >> 21) & 0x7F) as u8,
            ((value >> 14) & 0x7F) as u8,
            ((value >> 7) & 0x7F) as u8,
            (value & 0x7F) as u8,
        ]
    }

    fn id3_with_apic(picture: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.push(0); // codificación latin-1
        frame.extend_from_slice(b"image/jpeg\0");
        frame.push(3); // portada frontal
        frame.push(0); // descripción vacía
        frame.extend_from_slice(picture);

        let mut frames = Vec::new();
        frames.extend_from_slice(b"APIC");
        frames.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        frames.extend_from_slice(&[0, 0]);
        frames.extend_from_slice(&frame);

        let mut tag = Vec::new();
        tag.extend_from_slice(b"ID3\x03\x00\x00");
        tag.extend_from_slice(&syncsafe_bytes(frames.len() as u32));
        tag.extend_from_slice(&frames);
        tag
    }

    #[test]
    fn raw_rgba_requires_header_and_full_pixels() {
        let mut bytes = vec![8];
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let image = raw_rgba(&bytes).expect("cabecera válida");
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.pixels.len(), 8);

        assert!(raw_rgba(&bytes[..bytes.len() - 1]).is_none());
        assert!(raw_rgba(b"no es una portada").is_none());
        assert!(raw_rgba(&[8, 0, 0, 0, 0, 0, 0, 0, 0]).is_none());
    }

    #[test]
    fn apic_frame_yields_the_embedded_picture() {
        let tag = id3_with_apic(PICTURE);
        assert_eq!(id3_picture(&tag).as_deref(), Some(PICTURE));
    }

    #[test]
    fn tag_without_picture_or_truncated_is_rejected() {
        assert!(id3_picture(b"ID3\x03\x00\x00\x00\x00\x00\x00").is_none());
        assert!(id3_picture(b"no-id3").is_none());

        let mut tag = id3_with_apic(PICTURE);
        tag.truncate(tag.len() - 3);
        assert!(id3_picture(&tag).is_none());
    }

    #[test]
    fn aiff_id3_chunk_is_found_without_reading_audio() {
        let tag = id3_with_apic(PICTURE);

        let mut comm = Vec::new();
        comm.extend_from_slice(b"COMM");
        comm.extend_from_slice(&18u32.to_be_bytes());
        comm.extend_from_slice(&[0; 18]);

        let mut id3_chunk = Vec::new();
        id3_chunk.extend_from_slice(b"ID3 ");
        id3_chunk.extend_from_slice(&(tag.len() as u32).to_be_bytes());
        id3_chunk.extend_from_slice(&tag);
        if tag.len() % 2 == 1 {
            id3_chunk.push(0);
        }

        let mut body = Vec::new();
        body.extend_from_slice(b"AIFF");
        body.extend_from_slice(&comm);
        body.extend_from_slice(&id3_chunk);

        let mut file = Vec::new();
        file.extend_from_slice(b"FORM");
        file.extend_from_slice(&(body.len() as u32).to_be_bytes());
        file.extend_from_slice(&body);

        let path = temp_path("portada.aiff");
        std::fs::write(&path, &file).expect("escribir temporal");
        let found = embedded_image(&path);
        let _ = std::fs::remove_file(&path);

        assert_eq!(found.as_deref(), Some(PICTURE));
    }

    fn temp_path(name: &str) -> PathBuf {
        let unique = format!("dj-portadas-{}-{name}", std::process::id());
        std::env::temp_dir().join(unique)
    }
}
