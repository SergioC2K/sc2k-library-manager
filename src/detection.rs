//! Detección de librerías instaladas, como puerto neutral para la pantalla.
//!
//! La UI no conoce rutas ni formatos: solo consume `DetectedLibrary` y pide
//! abrir una candidata a través de `LibraryProvider`.
//!
//! `LocalProvider` localiza:
//! - Rekordbox: la exportación XML `rekordbox.xml` (la base `master.db` está
//!   cifrada y no se toca).
//! - Traktor: `collection.nml` de cada versión instalada.
//!
//! Antes de abrir se comprueba que el programa no esté en ejecución, para no
//! competir por los archivos. Todas las lecturas son de solo lectura.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::i18n::ES;
use crate::model::{DeckProgram, Library};
use crate::process_guard;
use crate::readers;

/// Identificador de una librería detectada.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DetectedLibraryId(pub u32);

/// Librería localizada en el equipo, todavía sin abrir.
#[derive(Clone, Debug)]
pub struct DetectedLibrary {
    pub id: DetectedLibraryId,
    pub program: DeckProgram,
    pub name: String,
    /// Texto mostrado al usuario; la UI no interpreta la ruta.
    pub location: String,
    /// `None` si contarlos exigiera abrir la librería completa.
    pub track_count: Option<usize>,
    pub playlist_count: Option<usize>,
    /// El programa está abierto: se avisa y se bloquea la apertura.
    pub program_running: bool,
    /// Aviso propio del formato (por ejemplo, que se lee una exportación).
    pub note: Option<String>,
}

/// Puerto de detección y apertura de librerías reales.
pub trait LibraryProvider: Send + Sync {
    /// Busca librerías instaladas. Debe ser rápido: no abrir las librerías completas.
    fn detect(&self) -> Vec<DetectedLibrary>;

    /// Abre una candidata e informa del progreso en 0..=1.
    /// La implementación corre en un hilo de fondo.
    fn open(&self, id: DetectedLibraryId, progress: &mut dyn FnMut(f32))
        -> Result<Library, String>;

    /// `true` si las candidatas son datos de ejemplo y no librerías reales.
    /// La pantalla lo avisa para no confundir al usuario.
    fn is_demo(&self) -> bool {
        false
    }
}

/// Proveedor de librerías locales reales.
#[derive(Default)]
pub struct LocalProvider {
    candidates: Mutex<Vec<LocalCandidate>>,
}

struct LocalCandidate {
    detected: DetectedLibrary,
    path: PathBuf,
}

impl LocalProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LibraryProvider for LocalProvider {
    fn detect(&self) -> Vec<DetectedLibrary> {
        let rekordbox_running = process_guard::any_running(process_guard::REKORDBOX_PROCESSES);
        let traktor_running = process_guard::any_running(process_guard::TRAKTOR_PROCESSES);

        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        let mut next_id = 0u32;

        for path in rekordbox_paths() {
            push_candidate(
                &mut candidates,
                &mut seen,
                &mut next_id,
                DeckProgram::Rekordbox,
                path,
                "Rekordbox".to_string(),
                rekordbox_running,
                Some(ES.rekordbox_xml_note.to_string()),
            );
        }

        for path in traktor_paths() {
            let name = path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("Traktor")
                .to_string();
            push_candidate(
                &mut candidates,
                &mut seen,
                &mut next_id,
                DeckProgram::Traktor,
                path,
                name,
                traktor_running,
                None,
            );
        }

        let detected: Vec<DetectedLibrary> = candidates
            .iter()
            .map(|candidate| candidate.detected.clone())
            .collect();

        let mut slot = self
            .candidates
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = candidates;
        detected
    }

    fn open(
        &self,
        id: DetectedLibraryId,
        progress: &mut dyn FnMut(f32),
    ) -> Result<Library, String> {
        let candidate = {
            let slot = self
                .candidates
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            slot.iter()
                .find(|candidate| candidate.detected.id == id)
                .map(|candidate| (candidate.detected.program, candidate.path.clone()))
        };
        let Some((program, path)) = candidate else {
            return Err(ES.unknown_library.to_string());
        };

        // Segunda comprobación justo antes de abrir: el programa pudo
        // arrancar entre la detección y la pulsación de "Abrir".
        let processes = match program {
            DeckProgram::Rekordbox => process_guard::REKORDBOX_PROCESSES,
            DeckProgram::Traktor => process_guard::TRAKTOR_PROCESSES,
            _ => return Err(ES.unsupported_program.to_string()),
        };
        if process_guard::any_running(processes) {
            return Err(crate::i18n::open_program_running(program.label()));
        }

        match program {
            DeckProgram::Rekordbox => readers::rekordbox_xml::read(&path, progress),
            DeckProgram::Traktor => readers::traktor::read(&path, progress),
            _ => Err(ES.unsupported_program.to_string()),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_candidate(
    candidates: &mut Vec<LocalCandidate>,
    seen: &mut HashSet<PathBuf>,
    next_id: &mut u32,
    program: DeckProgram,
    path: PathBuf,
    name: String,
    program_running: bool,
    note: Option<String>,
) {
    if !path.is_file() || !seen.insert(path.clone()) {
        return;
    }
    let detected = DetectedLibrary {
        id: DetectedLibraryId(*next_id),
        program,
        name,
        location: path.display().to_string(),
        track_count: None,
        playlist_count: None,
        program_running,
        note,
    };
    *next_id += 1;
    candidates.push(LocalCandidate { detected, path });
}

/// Rutas habituales de la exportación XML de Rekordbox.
fn rekordbox_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(windows)]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        paths.push(
            PathBuf::from(appdata)
                .join("Pioneer")
                .join("rekordbox")
                .join("rekordbox.xml"),
        );
    }

    if let Some(home) = home_dir() {
        paths.push(
            home.join("Library")
                .join("Pioneer")
                .join("rekordbox")
                .join("rekordbox.xml"),
        );
        paths.push(home.join("Music").join("rekordbox.xml"));
        paths.push(home.join("Music").join("Pioneer DJ").join("rekordbox.xml"));
        paths.push(home.join("Documents").join("rekordbox.xml"));
    }

    existing_files(paths)
}

/// Rutas habituales de `collection.nml` (todas las versiones instaladas).
fn traktor_paths() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Some(home) = home_dir() {
        bases.push(home.join("Documents").join("Native Instruments"));
        bases.push(home.join("Native Instruments"));
    }

    let mut found = Vec::new();
    for base in bases {
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        let mut folders: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        // Versión más reciente primero (orden alfabético inverso).
        folders.sort();
        folders.reverse();
        for folder in folders {
            let name = folder
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if !name.to_ascii_lowercase().starts_with("traktor") {
                continue;
            }
            let collection = folder.join("collection.nml");
            if collection.is_file() {
                found.push(collection);
            }
        }
    }

    dedupe(found)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn existing_files(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    dedupe(paths.into_iter().filter(|path| path.is_file()).collect())
}

fn dedupe(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

/// Proveedor de datos de ejemplo, solo para tests de la pantalla.
#[cfg(test)]
pub struct DemoProvider;

#[cfg(test)]
impl LibraryProvider for DemoProvider {
    fn detect(&self) -> Vec<DetectedLibrary> {
        vec![
            DetectedLibrary {
                id: DetectedLibraryId(1),
                program: DeckProgram::Rekordbox,
                name: "Rekordbox (ejemplo)".to_string(),
                location: "Ruta de ejemplo".to_string(),
                track_count: Some(crate::sample::DEMO_TRACK_COUNT),
                playlist_count: Some(8),
                program_running: false,
                note: None,
            },
            DetectedLibrary {
                id: DetectedLibraryId(2),
                program: DeckProgram::Traktor,
                name: "Traktor (ejemplo)".to_string(),
                location: "Ruta de ejemplo".to_string(),
                track_count: Some(crate::sample::DEMO_TRACK_COUNT),
                playlist_count: Some(8),
                program_running: false,
                note: None,
            },
        ]
    }

    fn open(
        &self,
        id: DetectedLibraryId,
        progress: &mut dyn FnMut(f32),
    ) -> Result<Library, String> {
        let program = match id.0 {
            1 => DeckProgram::Rekordbox,
            2 => DeckProgram::Traktor,
            _ => return Err("Librería de ejemplo desconocida".to_string()),
        };

        let mut library = crate::sample::demo_library_with_progress(progress);
        library.source = program;
        library.name = format!("{} (ejemplo)", program.label());
        Ok(library)
    }

    fn is_demo(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_provider_starts_without_candidates() {
        let provider = LocalProvider::new();
        assert!(provider.open(DetectedLibraryId(0), &mut |_| {}).is_err());
    }

    #[test]
    fn demo_provider_detects_rekordbox_and_traktor() {
        let detected = DemoProvider.detect();
        assert_eq!(detected.len(), 2);
        assert!(detected
            .iter()
            .any(|library| library.program == DeckProgram::Rekordbox));
        assert!(detected
            .iter()
            .any(|library| library.program == DeckProgram::Traktor));
        assert_ne!(detected[0].id, detected[1].id);
    }

    #[test]
    fn opening_a_demo_candidate_returns_the_requested_program() {
        let candidate = DemoProvider
            .detect()
            .into_iter()
            .find(|library| library.program == DeckProgram::Traktor)
            .expect("candidata de Traktor");

        let library = DemoProvider
            .open(candidate.id, &mut |_| {})
            .expect("abrir librería de ejemplo");

        assert_eq!(library.source, DeckProgram::Traktor);
        assert_eq!(library.tracks.len(), candidate.track_count.unwrap_or(0));
    }
}
