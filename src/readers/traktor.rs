//! Lector de la colección de Traktor (`collection.nml`).
//!
//! Es un XML abierto; se abre en solo lectura. Las playlists referencian las
//! pistas con `PRIMARYKEY`, cuya clave es la concatenación
//! `VOLUME + DIR + FILE` del `LOCATION` de cada entrada de la colección.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Duration;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use super::{attr, camelot_from_key_text, parse_bpm, parse_seconds, track_color_from_traktor_id};
use crate::model::{CoverSource, DeckProgram, Library, PlaylistId, PlaylistNode, Track, TrackId};

/// Lee una colección de Traktor.
///
/// `progress` recibe fracciones de 0 a 1 según avanza la lectura. El archivo
/// solo se abre en modo lectura.
pub fn read(path: &Path, progress: &mut dyn FnMut(f32)) -> Result<Library, String> {
    let file = File::open(path)
        .map_err(|error| format!("No se pudo abrir {}: {error}", path.display()))?;
    let total = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let base = path.parent().unwrap_or_else(|| Path::new(""));
    parse(BufReader::new(file), total, base, progress)
}

struct TrackDraft {
    track: Track,
    location: String,
    /// Identificador de portada de Traktor (`059/1BXR...`).
    coverart_id: Option<String>,
}

fn parse(
    source: impl BufRead,
    total_bytes: u64,
    base: &Path,
    progress: &mut dyn FnMut(f32),
) -> Result<Library, String> {
    let mut reader = Reader::from_reader(source);
    reader.trim_text(true);
    let mut buffer = Vec::new();

    let mut tracks: Vec<Track> = Vec::new();
    let mut by_location: HashMap<String, TrackId> = HashMap::new();
    let mut draft: Option<TrackDraft> = None;
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
                    if name != b"NML" {
                        return Err(
                            "El archivo no parece una colección de Traktor (NML).".to_string()
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
                    b"ENTRY" if in_collection => draft = Some(entry_from_element(&element)),
                    b"LOCATION" if in_collection => update_location(&mut draft, &element),
                    b"ALBUM" if in_collection => update_album(&mut draft, &element),
                    b"INFO" if in_collection => update_info(&mut draft, &element),
                    b"TEMPO" if in_collection => update_tempo(&mut draft, &element),
                    b"CUE_V2" if in_collection => count_cue(&mut draft, &element),
                    b"NODE" if in_playlists => start_node(&mut tree, &element),
                    b"PRIMARYKEY" => push_playlist_key(&mut tree, &element),
                    _ => {}
                }
            }
            Event::Empty(element) => {
                let name = element.local_name();
                match name.as_ref() {
                    b"COLLECTION" => saw_collection = true,
                    b"LOCATION" if in_collection => update_location(&mut draft, &element),
                    b"ALBUM" if in_collection => update_album(&mut draft, &element),
                    b"INFO" if in_collection => update_info(&mut draft, &element),
                    b"TEMPO" if in_collection => update_tempo(&mut draft, &element),
                    b"CUE_V2" if in_collection => count_cue(&mut draft, &element),
                    b"NODE" if in_playlists => {
                        start_node(&mut tree, &element);
                        tree.end(&by_location);
                    }
                    b"PRIMARYKEY" => push_playlist_key(&mut tree, &element),
                    _ => {}
                }
            }
            Event::End(element) => {
                let name = element.local_name();
                match name.as_ref() {
                    b"COLLECTION" => in_collection = false,
                    b"PLAYLISTS" => in_playlists = false,
                    b"ENTRY" if in_collection => {
                        if let Some(mut draft) = draft.take() {
                            let id = tracks.len();
                            if !draft.location.is_empty() {
                                by_location.insert(draft.location, id);
                            }
                            draft.track.id = id;
                            draft.track.cover = draft
                                .coverart_id
                                .as_deref()
                                .and_then(|coverart| coverart_path(base, coverart))
                                .map(CoverSource::RawRgba);
                            tracks.push(draft.track);
                        }
                    }
                    b"NODE" if in_playlists => tree.end(&by_location),
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }

        events += 1;
        if events % 2048 == 0 {
            report_progress(progress, reader.buffer_position() as u64, total_bytes);
        }
        buffer.clear();
    }

    if !saw_collection {
        return Err("La colección de Traktor no contiene pistas.".to_string());
    }

    report_progress(progress, total_bytes, total_bytes);

    Ok(Library {
        name: "Traktor".to_string(),
        source: DeckProgram::Traktor,
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

fn entry_from_element(element: &BytesStart<'_>) -> TrackDraft {
    TrackDraft {
        track: Track {
            id: 0,
            title: attr(element, b"TITLE").unwrap_or_default(),
            artist: attr(element, b"ARTIST").unwrap_or_default(),
            album: String::new(),
            bpm: 0.0,
            key: None,
            duration: Duration::ZERO,
            color: crate::model::TrackColor::None,
            cue_points: 0,
            cover: None,
        },
        location: String::new(),
        coverart_id: None,
    }
}

/// Ruta del archivo de portada de Traktor: `Coverart/<carpeta>/<id>000`.
fn coverart_path(base: &Path, coverart_id: &str) -> Option<std::path::PathBuf> {
    let (folder, name) = coverart_id.trim().split_once('/')?;
    if folder.is_empty() || name.is_empty() {
        return None;
    }
    Some(
        base.join("Coverart")
            .join(folder)
            .join(format!("{name}000")),
    )
}

fn update_location(draft: &mut Option<TrackDraft>, element: &BytesStart<'_>) {
    let Some(draft) = draft.as_mut() else {
        return;
    };
    let volume = attr(element, b"VOLUME").unwrap_or_default();
    let directory = attr(element, b"DIR").unwrap_or_default();
    let file = attr(element, b"FILE").unwrap_or_default();
    // Clave usada por Traktor en PRIMARYKEY: volumen + carpeta + archivo.
    draft.location = format!("{volume}{directory}{file}");
}

fn update_album(draft: &mut Option<TrackDraft>, element: &BytesStart<'_>) {
    if let Some(draft) = draft.as_mut() {
        draft.track.album = attr(element, b"TITLE").unwrap_or_default();
    }
}

fn update_info(draft: &mut Option<TrackDraft>, element: &BytesStart<'_>) {
    let Some(draft) = draft.as_mut() else {
        return;
    };
    if let Some(key) = attr(element, b"KEY")
        .as_deref()
        .and_then(camelot_from_key_text)
    {
        draft.track.key = Some(key);
    }
    if let Some(color) = attr(element, b"COLOR") {
        draft.track.color = track_color_from_traktor_id(&color);
    }
    if let Some(playtime) = attr(element, b"PLAYTIME") {
        draft.track.duration = Duration::from_secs(parse_seconds(&playtime));
    }
    if let Some(coverart) = attr(element, b"COVERARTID") {
        draft.coverart_id = Some(coverart);
    }
}

fn update_tempo(draft: &mut Option<TrackDraft>, element: &BytesStart<'_>) {
    if let Some(draft) = draft.as_mut() {
        if let Some(bpm) = attr(element, b"BPM") {
            draft.track.bpm = parse_bpm(&bpm);
        }
    }
}

/// Cuenta cues reales; la rejilla (`TYPE="4"`) y los bucles no cuentan.
fn count_cue(draft: &mut Option<TrackDraft>, element: &BytesStart<'_>) {
    let Some(draft) = draft.as_mut() else {
        return;
    };
    if attr(element, b"TYPE").as_deref() == Some("0") {
        draft.track.cue_points = draft.track.cue_points.saturating_add(1);
    }
}

fn push_playlist_key(tree: &mut PlaylistTree, element: &BytesStart<'_>) {
    // Solo interesan las referencias dentro de una playlist abierta.
    if matches!(element.local_name().as_ref(), b"PRIMARYKEY") {
        if let Some(key) = attr(element, b"KEY") {
            tree.add_track(key);
        }
    }
}

fn start_node(tree: &mut PlaylistTree, element: &BytesStart<'_>) {
    let kind = attr(element, b"TYPE").unwrap_or_default();
    let name = attr(element, b"NAME").unwrap_or_default();
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
        keys: Vec<String>,
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
            "PLAYLIST" => {
                let id = PlaylistId(self.next_id);
                self.next_id += 1;
                OpenNode::Playlist {
                    id,
                    name,
                    keys: Vec::new(),
                }
            }
            "FOLDER" if name == "$ROOT" && self.stack.is_empty() => OpenNode::Root {
                children: Vec::new(),
            },
            "FOLDER" => OpenNode::Folder {
                name,
                children: Vec::new(),
            },
            // Las listas inteligentes dependen de una consulta que no se
            // evalúa: se omiten en lugar de mostrarlas vacías.
            _ => OpenNode::Ignored,
        };
        self.stack.push(node);
    }

    fn add_track(&mut self, key: String) {
        if let Some(OpenNode::Playlist { keys, .. }) = self.stack.last_mut() {
            keys.push(key);
        }
    }

    fn end(&mut self, lookup: &HashMap<String, TrackId>) {
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
    use crate::model::TrackColor;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<NML VERSION="19">
  <HEAD COMPANY="Native Instruments" PROGRAM="Traktor"/>
  <MUSICFOLDERS/>
  <COLLECTION ENTRIES="2">
    <ENTRY TITLE="Primera" ARTIST="Artista A">
      <LOCATION DIR="/:Musica/:" FILE="primera.mp3" VOLUME="C:" VOLUMEID="abc"/>
      <ALBUM TITLE="Album A"/>
      <INFO KEY="8A" PLAYTIME="245" COLOR="1" COVERARTID="059/ABC"/>
      <TEMPO BPM="127.500000"/>
      <CUE_V2 NAME="n.n." TYPE="0" START="1.0"/>
      <CUE_V2 NAME="Grid" TYPE="4" START="0.0"/>
      <CUE_V2 NAME="Loop" TYPE="5" START="8.0"/>
    </ENTRY>
    <ENTRY TITLE="Segunda" ARTIST="Artista B">
      <LOCATION DIR="/:Musica/:" FILE="segunda.wav" VOLUME="D:" VOLUMEID="def"/>
      <INFO KEY="1d" PLAYTIME="300.4"/>
      <TEMPO BPM="0.000000"/>
    </ENTRY>
  </COLLECTION>
  <PLAYLISTS>
    <NODE TYPE="FOLDER" NAME="$ROOT">
      <SUBNODES COUNT="3">
        <NODE TYPE="PLAYLIST" NAME="Sesion">
          <PLAYLIST ENTRIES="2" TYPE="LIST">
            <ENTRY><PRIMARYKEY TYPE="TRACK" KEY="D:/:Musica/:segunda.wav"/></ENTRY>
            <ENTRY><PRIMARYKEY TYPE="TRACK" KEY="C:/:Musica/:primera.mp3"/></ENTRY>
          </PLAYLIST>
        </NODE>
        <NODE TYPE="SMARTLIST" NAME="Recientes"/>
        <NODE TYPE="FOLDER" NAME="Carpeta">
          <SUBNODES COUNT="1">
            <NODE TYPE="PLAYLIST" NAME="Vacia"><PLAYLIST ENTRIES="0"/></NODE>
          </SUBNODES>
        </NODE>
      </SUBNODES>
    </NODE>
  </PLAYLISTS>
</NML>"#;

    fn parse_sample() -> Library {
        let bytes = SAMPLE.as_bytes();
        parse(bytes, bytes.len() as u64, Path::new(""), &mut |_| {}).expect("NML de muestra válido")
    }

    #[test]
    fn entries_are_read_with_metadata() {
        let library = parse_sample();
        assert_eq!(library.source, DeckProgram::Traktor);
        assert_eq!(library.tracks.len(), 2);
        assert_eq!(library.tracks[0].id, 0);
        assert_eq!(library.tracks[1].id, 1);

        let first = &library.tracks[0];
        assert_eq!(first.title, "Primera");
        assert_eq!(first.artist, "Artista A");
        assert_eq!(first.album, "Album A");
        assert_eq!(first.bpm, 127.5);
        assert_eq!(first.key, Some(CamelotKey::new(8, KeyMode::Minor).unwrap()));
        assert_eq!(first.duration, Duration::from_secs(245));
        assert_eq!(first.color, TrackColor::Red);
        assert_eq!(first.cue_points, 1);
        assert!(matches!(first.cover, Some(CoverSource::RawRgba(_))));

        assert_eq!(
            library.tracks[1].key,
            Some(CamelotKey::new(8, KeyMode::Major).unwrap())
        );
        assert!(library.tracks[1].cover.is_none());
    }

    #[test]
    fn coverart_ids_resolve_inside_the_collection_folder() {
        let path = coverart_path(Path::new("C:/Traktor"), "059/ABC").expect("ruta de portada");
        let text = path.to_string_lossy();
        assert!(text.contains("Coverart"));
        assert!(text.ends_with("ABC000"));
        assert!(coverart_path(Path::new(""), "sin-barra").is_none());
        assert!(coverart_path(Path::new(""), "/vacio").is_none());
    }

    #[test]
    fn playlists_resolve_primary_keys_and_skip_smartlists() {
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

        assert!(library
            .playlists
            .iter()
            .all(|node| node.name() != "Recientes"));
    }

    #[test]
    fn non_traktor_xml_is_rejected() {
        assert!(parse(
            "<DJ_PLAYLISTS></DJ_PLAYLISTS>".as_bytes(),
            0,
            Path::new(""),
            &mut |_| {}
        )
        .is_err());
    }
}
