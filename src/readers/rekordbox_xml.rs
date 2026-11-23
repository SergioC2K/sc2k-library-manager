//! Lector de la exportación XML de Rekordbox (`rekordbox.xml`).
//!
//! Rekordbox 6 guarda su colección en `master.db`, que está cifrada; la vía
//! abierta y documentada es la exportación XML (`Archivo > Exportar colección
//! en formato xml`). Este lector abre ese archivo en solo lectura.
//!
//! Estructura que se interpreta:
//! - `COLLECTION/TRACK`: pistas con `TrackID`, título, artista, álbum, BPM,
//!   tonalidad, duración y color.
//! - `TRACK/POSITION_MARK`: puntos de cue (se descarta la rejilla automática).
//! - `PLAYLISTS/NODE`: carpetas (`Type="0"`) y playlists (`Type="1"`), con
//!   `TRACK Key="..."` dentro de cada playlist.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use super::{
    attr, camelot_from_key_text, file_url_to_path, parse_bpm, parse_seconds, track_color_from_hex,
};
use crate::model::{CoverSource, DeckProgram, Library, PlaylistId, PlaylistNode, Track, TrackId};

/// Lee una exportación XML de Rekordbox.
///
/// `progress` recibe fracciones de 0 a 1 según avanza la lectura. El archivo
/// solo se abre en modo lectura.
pub fn read(path: &Path, progress: &mut dyn FnMut(f32)) -> Result<Library, String> {
    let file = File::open(path)
        .map_err(|error| format!("No se pudo abrir {}: {error}", path.display()))?;
    let total = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    parse(BufReader::new(file), total, progress)
}

fn parse(
    source: impl BufRead,
    total_bytes: u64,
    progress: &mut dyn FnMut(f32),
) -> Result<Library, String> {
    let mut reader = Reader::from_reader(source);
    reader.trim_text(true);
    let mut buffer = Vec::new();

    let mut tracks: Vec<Track> = Vec::new();
    let mut by_track_id: HashMap<u64, TrackId> = HashMap::new();
    let mut current: Option<Track> = None;
    let mut current_track_id: Option<u64> = None;
    let mut tree = PlaylistTree::new();

    let mut saw_root = false;
    let mut saw_collection = false;
    let mut in_collection = false;
    let mut in_playlists = false;
    let mut events: u64 = 0;

    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| format!("XML no válido: {error}"))?;

        match event {
            Event::Start(element) => {
                let name = element.local_name();
                let name = name.as_ref();
                if !saw_root {
                    if name != b"DJ_PLAYLISTS" {
                        return Err(
                            "El archivo no parece una exportación XML de Rekordbox.".to_string()
                        );
                    }
                    saw_root = true;
                }

                match name {
                    b"COLLECTION" => {
                        saw_collection = true;
                        in_collection = true;
                    }
                    b"PLAYLISTS" => in_playlists = true,
                    b"TRACK" if in_playlists => push_playlist_key(&mut tree, &element),
                    b"TRACK" => {
                        let id = attr(&element, b"TrackID").and_then(|text| text.parse().ok());
                        current_track_id = id;
                        current = Some(track_from_element(&element, tracks.len()));
                    }
                    b"POSITION_MARK" => count_cue(&mut current, &element),
                    b"NODE" if in_playlists => start_node(&mut tree, &element),
                    _ => {}
                }
            }
            Event::Empty(element) => {
                let name = element.local_name();
                match name.as_ref() {
                    b"COLLECTION" => saw_collection = true,
                    b"TRACK" if in_playlists => push_playlist_key(&mut tree, &element),
                    b"TRACK" => {
                        let track = track_from_element(&element, tracks.len());
                        let id = attr(&element, b"TrackID").and_then(|text| text.parse().ok());
                        finish_track(&mut tracks, &mut by_track_id, track, id);
                    }
                    b"POSITION_MARK" => count_cue(&mut current, &element),
                    b"NODE" if in_playlists => {
                        start_node(&mut tree, &element);
                        tree.end(&by_track_id);
                    }
                    _ => {}
                }
            }
            Event::End(element) => {
                let name = element.local_name();
                match name.as_ref() {
                    b"COLLECTION" => in_collection = false,
                    b"PLAYLISTS" => in_playlists = false,
                    b"TRACK" if in_collection => {
                        if let Some(track) = current.take() {
                            let id = current_track_id.take();
                            finish_track(&mut tracks, &mut by_track_id, track, id);
                        }
                    }
                    b"NODE" if in_playlists => tree.end(&by_track_id),
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }

        events += 1;
        if events % 512 == 0 {
            report_progress(progress, reader.buffer_position() as u64, total_bytes);
        }
        buffer.clear();
    }

    if !saw_collection {
        return Err("La exportación XML no contiene una colección.".to_string());
    }

    report_progress(progress, total_bytes, total_bytes);

    Ok(Library {
        name: "Rekordbox".to_string(),
        source: DeckProgram::Rekordbox,
        tracks,
        playlists: tree.finish(),
    })
}

fn report_progress(progress: &mut dyn FnMut(f32), position: u64, total: u64) {
    let fraction = if total == 0 {
        0.0
    } else {
        (position as f32 / total as f32).clamp(0.0, 1.0)
    };
    progress(fraction);
}

fn finish_track(
    tracks: &mut Vec<Track>,
    by_track_id: &mut HashMap<u64, TrackId>,
    track: Track,
    source_id: Option<u64>,
) {
    if let Some(source_id) = source_id {
        by_track_id.insert(source_id, tracks.len());
    }
    tracks.push(track);
}

fn track_from_element(element: &BytesStart<'_>, index: usize) -> Track {
    let key = attr(element, b"Tonality")
        .as_deref()
        .and_then(camelot_from_key_text);
    let color = attr(element, b"Colour")
        .map(|value| track_color_from_hex(&value))
        .unwrap_or_default();
    // La exportación no incluye la ruta de la portada: el arte vive incrustado
    // en el propio archivo de audio.
    let cover = attr(element, b"Location")
        .as_deref()
        .and_then(file_url_to_path)
        .map(CoverSource::Embedded);

    Track {
        id: index,
        title: attr(element, b"Name").unwrap_or_default(),
        artist: attr(element, b"Artist").unwrap_or_default(),
        album: attr(element, b"Album").unwrap_or_default(),
        bpm: attr(element, b"AverageBpm")
            .map(|value| parse_bpm(&value))
            .unwrap_or(0.0),
        key,
        duration: Duration::from_secs(
            attr(element, b"TotalTime")
                .map(|value| parse_seconds(&value))
                .unwrap_or(0),
        ),
        color,
        cue_points: 0,
        cover,
    }
}

/// Cuenta cues reales: se descarta la rejilla automática (`AutoGrid`).
fn count_cue(track: &mut Option<Track>, element: &BytesStart<'_>) {
    let Some(track) = track.as_mut() else {
        return;
    };
    let kind = attr(element, b"Type").unwrap_or_default();
    let name = attr(element, b"Name").unwrap_or_default();
    if kind == "0" && !name.eq_ignore_ascii_case("AutoGrid") {
        track.cue_points = track.cue_points.saturating_add(1);
    }
}

fn push_playlist_key(tree: &mut PlaylistTree, element: &BytesStart<'_>) {
    if let Some(key) = attr(element, b"Key").and_then(|text| text.parse::<u64>().ok()) {
        tree.add_track(key);
    }
}

fn start_node(tree: &mut PlaylistTree, element: &BytesStart<'_>) {
    let kind = attr(element, b"Type").unwrap_or_default();
    let name = attr(element, b"Name").unwrap_or_default();
    tree.start(&kind, name);
}

enum OpenNode {
    Root {
        children: Vec<PlaylistNode>,
    },
    Folder {
        name: String,
        children: Vec<PlaylistNode>,
    },
    Playlist {
        id: PlaylistId,
        name: String,
        keys: Vec<u64>,
    },
    Ignored,
}

struct PlaylistTree {
    roots: Vec<PlaylistNode>,
    stack: Vec<OpenNode>,
    next_id: u32,
}

impl PlaylistTree {
    fn new() -> Self {
        Self {
            roots: Vec::new(),
            stack: Vec::new(),
            next_id: 0,
        }
    }

    fn start(&mut self, kind: &str, name: String) {
        let node = match kind {
            "1" => {
                let id = PlaylistId(self.next_id);
                self.next_id += 1;
                OpenNode::Playlist {
                    id,
                    name,
                    keys: Vec::new(),
                }
            }
            "0" if name == "ROOT" && self.stack.is_empty() => OpenNode::Root {
                children: Vec::new(),
            },
            "0" => OpenNode::Folder {
                name,
                children: Vec::new(),
            },
            _ => OpenNode::Ignored,
        };
        self.stack.push(node);
    }

    fn add_track(&mut self, key: u64) {
        if let Some(OpenNode::Playlist { keys, .. }) = self.stack.last_mut() {
            keys.push(key);
        }
    }

    fn end(&mut self, lookup: &HashMap<u64, TrackId>) {
        let Some(node) = self.stack.pop() else {
            return;
        };
        match node {
            OpenNode::Playlist { id, name, keys } => {
                let track_ids = keys
                    .into_iter()
                    .filter_map(|key| lookup.get(&key).copied())
                    .collect();
                self.push(PlaylistNode::Playlist {
                    id,
                    name,
                    track_ids,
                });
            }
            OpenNode::Folder { name, children } => {
                self.push(PlaylistNode::Folder { name, children });
            }
            OpenNode::Root { children } => self.roots = children,
            OpenNode::Ignored => {}
        }
    }

    fn push(&mut self, node: PlaylistNode) {
        match self.stack.last_mut() {
            Some(OpenNode::Root { children }) | Some(OpenNode::Folder { children, .. }) => {
                children.push(node);
            }
            _ => self.roots.push(node),
        }
    }

    fn finish(self) -> Vec<PlaylistNode> {
        self.roots
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harmony::{CamelotKey, KeyMode};

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<DJ_PLAYLISTS Version="1.0.0">
  <PRODUCT Name="rekordbox" Version="6.8.5" Company="Pioneer DJ"/>
  <COLLECTION Entries="2">
    <TRACK TrackID="10" Name="Uno &amp; dos" Artist="Artista" Album="Album" TotalTime="245" AverageBpm="128.50" Tonality="8A" Colour="0xFF0000" Location="file://localhost/C:/Musica/Uno%20dos.mp3">
      <POSITION_MARK Name="" Type="0" Start="0.1" Num="0"/>
      <POSITION_MARK Name="AutoGrid" Type="0" Start="0.0" Num="-1"/>
      <POSITION_MARK Name="" Type="4" Start="16.0" Num="0"/>
    </TRACK>
    <TRACK TrackID="20" Name="Dos" Artist="Otro" Album="" TotalTime="300" AverageBpm="0.00" Tonality="1d"/>
  </COLLECTION>
  <PLAYLISTS>
    <NODE Type="0" Name="ROOT" Count="2">
      <NODE Name="Sesion" Type="1" KeyType="0" Entries="2">
        <TRACK Key="20"/>
        <TRACK Key="10"/>
      </NODE>
      <NODE Type="0" Name="Carpeta" Count="1">
        <NODE Name="Vacia" Type="1" KeyType="0" Entries="0"/>
      </NODE>
    </NODE>
  </PLAYLISTS>
</DJ_PLAYLISTS>"#;

    fn parse_sample() -> Library {
        let bytes = SAMPLE.as_bytes();
        parse(bytes, bytes.len() as u64, &mut |_| {}).expect("XML de muestra válido")
    }

    #[test]
    fn tracks_are_read_with_metadata() {
        let library = parse_sample();
        assert_eq!(library.source, DeckProgram::Rekordbox);
        assert_eq!(library.tracks.len(), 2);

        let first = &library.tracks[0];
        assert_eq!(first.title, "Uno & dos");
        assert_eq!(first.artist, "Artista");
        assert_eq!(first.album, "Album");
        assert_eq!(first.bpm, 128.5);
        assert_eq!(first.key, Some(CamelotKey::new(8, KeyMode::Minor).unwrap()));
        assert_eq!(first.duration, Duration::from_secs(245));
        assert_eq!(first.color, crate::model::TrackColor::Red);
        // Solo el cue real: ni AutoGrid ni el marcador de loop.
        assert_eq!(first.cue_points, 1);

        assert_eq!(
            library.tracks[1].key,
            Some(CamelotKey::new(8, KeyMode::Major).unwrap())
        );
    }

    #[test]
    fn location_attribute_points_to_the_embedded_cover() {
        let library = parse_sample();
        match &library.tracks[0].cover {
            Some(CoverSource::Embedded(path)) => {
                assert!(path.to_string_lossy().ends_with("Uno dos.mp3"));
            }
            other => panic!("se esperaba arte incrustado, hay {other:?}"),
        }
        assert!(library.tracks[1].cover.is_none());
    }

    #[test]
    fn playlists_and_nested_folders_are_rebuilt() {
        let library = parse_sample();
        assert_eq!(library.playlist_count(), 2);

        let sesion = library
            .playlists
            .iter()
            .find(|node| node.name() == "Sesion")
            .expect("playlist Sesion");
        match sesion {
            PlaylistNode::Playlist { track_ids, .. } => assert_eq!(track_ids, &[1, 0]),
            PlaylistNode::Folder { .. } => panic!("Sesion deberia ser una playlist"),
        }

        let carpeta = library
            .playlists
            .iter()
            .find(|node| node.name() == "Carpeta")
            .expect("carpeta");
        match carpeta {
            PlaylistNode::Folder { children, .. } => {
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].name(), "Vacia");
            }
            PlaylistNode::Playlist { .. } => panic!("Carpeta deberia ser una carpeta"),
        }
    }

    #[test]
    fn unknown_playlist_keys_are_ignored() {
        let xml = r#"<DJ_PLAYLISTS><COLLECTION Entries="0"/><PLAYLISTS>
            <NODE Type="0" Name="ROOT"><NODE Name="P" Type="1"><TRACK Key="99"/></NODE></NODE>
            </PLAYLISTS></DJ_PLAYLISTS>"#;
        let library = parse(xml.as_bytes(), 0, &mut |_| {}).expect("XML válido");
        assert!(library.tracks.is_empty());
        assert_eq!(library.playlist_track_ids(PlaylistId(0)), Some(&[][..]));
    }

    #[test]
    fn non_rekordbox_xml_is_rejected() {
        let xml = r#"<PLAYLISTS></PLAYLISTS>"#;
        assert!(parse(xml.as_bytes(), 0, &mut |_| {}).is_err());
    }
}
