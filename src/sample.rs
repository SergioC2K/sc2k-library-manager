//! Datos de ejemplo, deterministas, mientras no exista el lector real.
//!
//! TODO: sustituir por una carga real desde Rekordbox/Serato/Traktor/Virtual DJ/
//! Engine DJ. Este modulo solo existe para que la pantalla sea navegable y para
//! comprobar el rendimiento con una libreria grande (50 000 pistas).

use std::time::Duration;

use crate::harmony::{CamelotKey, KeyMode};
use crate::model::{DeckProgram, Library, PlaylistId, PlaylistNode, Track, TrackColor, TrackId};

/// Tamano de la libreria de ejemplo.
pub const DEMO_TRACK_COUNT: usize = 50_000;

const ADJECTIVES: [&str; 12] = [
    "Neon", "Midnight", "Solar", "Velvet", "Chrome", "Ivory", "Hidden", "Electric", "Lunar",
    "Crystal", "Silent", "Golden",
];

const NOUNS: [&str; 12] = [
    "District", "Rooftop", "Echo", "Tide", "Avenue", "Mirror", "Signal", "Harbor", "Garden",
    "Circuit", "Fever", "Orbit",
];

const VARIANTS: [&str; 4] = ["", "(Mix)", "(Dub)", "(Live)"];

const ARTISTS: [&str; 12] = [
    "Ana Soler",
    "Bastian Cruz",
    "Carla Nova",
    "Damian Rios",
    "Elena Mar",
    "Franco Vega",
    "Gala Indigo",
    "Hugo Lumen",
    "Iris Cano",
    "Jota Pereira",
    "Karla Sur",
    "Leo Marea",
];

const ALBUMS: [&str; 10] = [
    "Primera Luz",
    "Nocturno",
    "Cartografia",
    "Marea Alta",
    "Interior",
    "Rutas",
    "Vertigo",
    "Aurora",
    "Pulso",
    "Deriva",
];

const COLORS: [TrackColor; 7] = [
    TrackColor::Pink,
    TrackColor::Red,
    TrackColor::Orange,
    TrackColor::Yellow,
    TrackColor::Green,
    TrackColor::Blue,
    TrackColor::Purple,
];

/// Genera la libreria de ejemplo e informa del progreso en 0..=1.
pub fn demo_library_with_progress(mut progress: impl FnMut(f32)) -> Library {
    let mut rng = Lcg::new(0x5EED_1024);
    let mut tracks = Vec::with_capacity(DEMO_TRACK_COUNT);

    for index in 0..DEMO_TRACK_COUNT {
        let adjective = ADJECTIVES[index % ADJECTIVES.len()];
        let noun = NOUNS[(index / ADJECTIVES.len()) % NOUNS.len()];
        let variant = VARIANTS[(index / (ADJECTIVES.len() * NOUNS.len())) % VARIANTS.len()];
        let title = if variant.is_empty() {
            format!("{adjective} {noun}")
        } else {
            format!("{adjective} {noun} {variant}")
        };

        let number = (rng.next_u32() % 12 + 1) as u8;
        let mode = if rng.next_u32() % 2 == 0 {
            KeyMode::Minor
        } else {
            KeyMode::Major
        };
        // Los lectores reales entregan la clave como texto ("8A"); usamos el
        // mismo parser para que el formato de ejemplo sea representativo.
        // Una de cada once pistas no trae clave, como ocurre en librerias reales.
        let key = if index % 11 == 0 {
            None
        } else {
            CamelotKey::parse(&format!("{number}{}", mode.letter()))
        };

        let bpm = 60.0 + (rng.next_u32() % 1_201) as f32 / 10.0;
        let seconds = u64::from(120 + rng.next_u32() % 300);
        let cue_points = (rng.next_u32() % 9) as u8;
        let color = if index % 9 == 0 {
            TrackColor::None
        } else {
            COLORS[(rng.next_u32() as usize) % COLORS.len()]
        };

        tracks.push(Track {
            id: index,
            title,
            artist: ARTISTS[(rng.next_u32() as usize) % ARTISTS.len()].to_string(),
            album: ALBUMS[(rng.next_u32() as usize) % ALBUMS.len()].to_string(),
            bpm,
            key,
            duration: Duration::from_secs(seconds),
            color,
            cue_points,
            cover: None,
        });

        if index % 1_000 == 0 {
            progress(index as f32 / DEMO_TRACK_COUNT as f32);
        }
    }
    progress(1.0);

    let playlists = build_playlists(&tracks);
    Library {
        name: "Libreria de ejemplo".to_string(),
        source: DeckProgram::Rekordbox,
        tracks,
        playlists,
    }
}

fn build_playlists(tracks: &[Track]) -> Vec<PlaylistNode> {
    let mut next_id = 0;
    let friday = playlist(
        "Viernes noche",
        &mut next_id,
        (0..tracks.len()).step_by(17).take(1_800),
    );
    let residency = playlist(
        "Residencia de verano",
        &mut next_id,
        (5..tracks.len()).step_by(23).take(1_200),
    );
    let ceremony = playlist(
        "Ceremonia",
        &mut next_id,
        (40..tracks.len()).step_by(37).take(180),
    );
    let banquet = playlist(
        "Banquete",
        &mut next_id,
        (11..tracks.len()).step_by(41).take(260),
    );
    let new_arrivals = playlist(
        "Nuevos ingresos",
        &mut next_id,
        tracks.len().saturating_sub(240)..tracks.len(),
    );
    let no_key: Vec<TrackId> = tracks
        .iter()
        .filter(|track| track.key.is_none())
        .map(|track| track.id)
        .take(900)
        .collect();
    let without_key = playlist("Sin clave", &mut next_id, no_key);
    let favourites = playlist(
        "Favoritas",
        &mut next_id,
        (0..tracks.len()).step_by(97).take(120),
    );
    let radio = playlist(
        "Seleccion del mes",
        &mut next_id,
        (0..tracks.len()).step_by(131).take(80),
    );

    vec![
        PlaylistNode::Folder {
            name: "Sesiones".to_string(),
            children: vec![
                friday,
                residency,
                PlaylistNode::Folder {
                    name: "Boda".to_string(),
                    children: vec![ceremony, banquet],
                },
            ],
        },
        PlaylistNode::Folder {
            name: "Gestion".to_string(),
            children: vec![new_arrivals, without_key, favourites],
        },
        PlaylistNode::Folder {
            name: "Radio".to_string(),
            children: vec![radio],
        },
    ]
}

fn playlist(
    name: &str,
    next_id: &mut u32,
    track_ids: impl IntoIterator<Item = TrackId>,
) -> PlaylistNode {
    let id = PlaylistId(*next_id);
    *next_id += 1;
    PlaylistNode::Playlist {
        id,
        name: name.to_string(),
        track_ids: track_ids.into_iter().collect(),
    }
}

/// Generador congruencial lineal: reproducible y sin dependencias.
struct Lcg {
    state: u32,
}

impl Lcg {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_library_is_consistent() {
        let library = demo_library_with_progress(|_| {});
        assert_eq!(library.tracks.len(), DEMO_TRACK_COUNT);
        assert!(library.playlist_count() >= 8);
        assert!(library
            .tracks
            .iter()
            .all(|track| track.id < DEMO_TRACK_COUNT));
        assert!(library
            .tracks
            .iter()
            .all(|track| (60.0..=180.0).contains(&track.bpm)));
        for node in &library.playlists {
            collect_ids(node, &library);
        }
    }

    fn collect_ids(node: &PlaylistNode, library: &Library) {
        match node {
            PlaylistNode::Folder { children, .. } => {
                for child in children {
                    collect_ids(child, library);
                }
            }
            PlaylistNode::Playlist { track_ids, .. } => {
                assert!(track_ids.iter().all(|id| library.tracks.get(*id).is_some()));
            }
        }
    }
}
