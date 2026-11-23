//! Modelo neutral que consume la pantalla.
//!
//! TODO(arquitectura): sustituir este modelo provisional por `core::Library`,
//! `core::Track` y `core::PlaylistNode` cuando exista el crate `core`.
//! La UI no debe conocer ningun formato de DJ: solo este modelo.

use std::path::PathBuf;
use std::time::Duration;

use egui::Color32;

use crate::harmony::CamelotKey;

/// Indice de una pista dentro de `Library::tracks`.
pub type TrackId = usize;

/// Programa de DJ de origen o destino de una conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeckProgram {
    #[default]
    Rekordbox,
    Serato,
    Traktor,
    VirtualDj,
    EngineDj,
}

impl DeckProgram {
    pub const ALL: [Self; 5] = [
        Self::Rekordbox,
        Self::Serato,
        Self::Traktor,
        Self::VirtualDj,
        Self::EngineDj,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Rekordbox => "Rekordbox",
            Self::Serato => "Serato DJ",
            Self::Traktor => "Traktor",
            Self::VirtualDj => "Virtual DJ",
            Self::EngineDj => "Engine DJ",
        }
    }
}

/// Color de pista usado por las librerias de DJ.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackColor {
    #[default]
    None,
    Pink,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
}

impl TrackColor {
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "Sin color",
            Self::Pink => "Rosa",
            Self::Red => "Rojo",
            Self::Orange => "Naranja",
            Self::Yellow => "Amarillo",
            Self::Green => "Verde",
            Self::Blue => "Azul",
            Self::Purple => "Morado",
        }
    }

    /// Color de dibujo, o `None` si la pista no tiene color asignado.
    pub const fn swatch(self) -> Option<Color32> {
        let rgb = match self {
            Self::None => return None,
            Self::Pink => (240, 120, 190),
            Self::Red => (226, 76, 76),
            Self::Orange => (236, 148, 62),
            Self::Yellow => (226, 202, 74),
            Self::Green => (96, 190, 112),
            Self::Blue => (84, 150, 230),
            Self::Purple => (158, 122, 224),
        };
        Some(Color32::from_rgb(rgb.0, rgb.1, rgb.2))
    }
}

/// De dónde salen los píxeles de la portada. Es un detalle de almacenamiento,
/// no un formato de DJ: la UI solo lo pasa al hilo que decodifica portadas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoverSource {
    /// Imagen incrustada en el propio archivo de audio (`APIC` de ID3v2).
    Embedded(PathBuf),
    /// RGBA sin comprimir con cabecera de 9 bytes (así la guarda Traktor).
    RawRgba(PathBuf),
}

/// Pista del modelo neutral.
#[derive(Clone, Debug)]
pub struct Track {
    pub id: TrackId,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub bpm: f32,
    pub key: Option<CamelotKey>,
    pub duration: Duration,
    pub color: TrackColor,
    pub cue_points: u8,
    /// Portada real; si falta, `covers` dibuja el sustituto con la inicial.
    pub cover: Option<CoverSource>,
}

impl Track {
    /// Duracion en formato `m:ss` o `h:mm:ss`.
    pub fn duration_label(&self) -> String {
        let total = self.duration.as_secs();
        let seconds = total % 60;
        let minutes = (total / 60) % 60;
        let hours = total / 3600;
        if hours > 0 {
            format!("{hours}:{minutes:02}:{seconds:02}")
        } else {
            format!("{minutes}:{seconds:02}")
        }
    }
}

/// Identificador estable de playlist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaylistId(pub u32);

/// Arbol de carpetas y playlists.
#[derive(Clone, Debug)]
pub enum PlaylistNode {
    Folder {
        name: String,
        children: Vec<Self>,
    },
    Playlist {
        id: PlaylistId,
        name: String,
        track_ids: Vec<TrackId>,
    },
}

impl PlaylistNode {
    pub fn name(&self) -> &str {
        match self {
            Self::Folder { name, .. } | Self::Playlist { name, .. } => name,
        }
    }
}

/// Seleccion activa en el panel izquierdo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Selection {
    #[default]
    AllTracks,
    Playlist(PlaylistId),
}

/// Libreria cargada (de momento, datos de ejemplo).
#[derive(Clone, Debug)]
pub struct Library {
    pub name: String,
    pub source: DeckProgram,
    pub tracks: Vec<Track>,
    pub playlists: Vec<PlaylistNode>,
}

impl Library {
    // TODO: construir desde los lectores reales de Rekordbox/Serato/Traktor.
    pub fn playlist(&self, id: PlaylistId) -> Option<&PlaylistNode> {
        find_playlist(&self.playlists, id)
    }

    pub fn playlist_track_ids(&self, id: PlaylistId) -> Option<&[TrackId]> {
        match self.playlist(id)? {
            PlaylistNode::Playlist { track_ids, .. } => Some(track_ids),
            PlaylistNode::Folder { .. } => None,
        }
    }

    pub fn playlist_name(&self, id: PlaylistId) -> Option<&str> {
        self.playlist(id).map(PlaylistNode::name)
    }

    /// Numero de pistas de la seleccion (antes de filtros de busqueda/armonia).
    pub fn selection_len(&self, selection: Selection) -> usize {
        match selection {
            Selection::AllTracks => self.tracks.len(),
            Selection::Playlist(id) => self.playlist_track_ids(id).map_or(0, <[TrackId]>::len),
        }
    }

    /// Numero total de playlists (las carpetas no cuentan).
    pub fn playlist_count(&self) -> usize {
        count_playlists(&self.playlists)
    }
}

fn find_playlist(nodes: &[PlaylistNode], wanted: PlaylistId) -> Option<&PlaylistNode> {
    for node in nodes {
        match node {
            PlaylistNode::Folder { children, .. } => {
                if let Some(found) = find_playlist(children, wanted) {
                    return Some(found);
                }
            }
            PlaylistNode::Playlist { id, .. } if *id == wanted => return Some(node),
            PlaylistNode::Playlist { .. } => {}
        }
    }
    None
}

fn count_playlists(nodes: &[PlaylistNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            PlaylistNode::Folder { children, .. } => count_playlists(children),
            PlaylistNode::Playlist { .. } => 1,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(seconds: u64) -> Track {
        Track {
            id: 0,
            title: "Titulo".to_string(),
            artist: "Artista".to_string(),
            album: "Album".to_string(),
            bpm: 124.0,
            key: None,
            duration: Duration::from_secs(seconds),
            color: TrackColor::None,
            cue_points: 0,
            cover: None,
        }
    }

    #[test]
    fn duration_label_formats_minutes_and_hours() {
        assert_eq!(track(45).duration_label(), "0:45");
        assert_eq!(track(342).duration_label(), "5:42");
        assert_eq!(track(3_723).duration_label(), "1:02:03");
    }

    #[test]
    fn nested_playlists_are_found_and_counted() {
        let library = Library {
            name: "Demo".to_string(),
            source: DeckProgram::Rekordbox,
            tracks: vec![],
            playlists: vec![
                PlaylistNode::Folder {
                    name: "Sesiones".to_string(),
                    children: vec![
                        PlaylistNode::Playlist {
                            id: PlaylistId(0),
                            name: "Viernes".to_string(),
                            track_ids: vec![0, 1],
                        },
                        PlaylistNode::Folder {
                            name: "Boda".to_string(),
                            children: vec![PlaylistNode::Playlist {
                                id: PlaylistId(1),
                                name: "Ceremonia".to_string(),
                                track_ids: vec![2],
                            }],
                        },
                    ],
                },
                PlaylistNode::Playlist {
                    id: PlaylistId(2),
                    name: "Nuevos".to_string(),
                    track_ids: vec![3, 4, 5],
                },
            ],
        };

        assert_eq!(library.playlist_count(), 3);
        assert_eq!(library.playlist_track_ids(PlaylistId(1)), Some(&[2][..]));
        assert_eq!(library.playlist_name(PlaylistId(2)), Some("Nuevos"));
        assert_eq!(library.selection_len(Selection::Playlist(PlaylistId(2))), 3);
        assert_eq!(library.selection_len(Selection::AllTracks), 0);
        assert!(library.playlist_track_ids(PlaylistId(9)).is_none());
    }

    #[test]
    fn track_colors_map_to_swatches() {
        assert!(TrackColor::None.swatch().is_none());
        assert!(TrackColor::Red.swatch().is_some());
        assert_eq!(TrackColor::None.label(), "Sin color");
    }
}
