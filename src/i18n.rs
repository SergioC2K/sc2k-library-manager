//! Textos de la interfaz.
//!
//! Todos los literales visibles viven aquí para poder añadir inglés más tarde
//! sin tocar la lógica de dibujo.

pub struct Texts {
    pub app_title: &'static str,
    pub menu_file: &'static str,
    pub detect_libraries: &'static str,
    pub detect_title: &'static str,
    pub detect_hint: &'static str,
    pub detect_scanning: &'static str,
    pub detect_none: &'static str,
    pub detect_none_hint: &'static str,
    pub detect_rescan: &'static str,
    pub detect_demo_note: &'static str,
    pub demo_tag: &'static str,
    pub count_at_open: &'static str,
    pub program_running_note: &'static str,
    pub rekordbox_xml_note: &'static str,
    pub unknown_library: &'static str,
    pub unsupported_program: &'static str,
    pub open: &'static str,
    pub cancel: &'static str,
    pub status_detecting: &'static str,
    pub status_worker_lost: &'static str,
    pub export: &'static str,
    pub quit: &'static str,
    pub menu_view: &'static str,
    pub view_list: &'static str,
    pub view_grid: &'static str,
    pub view_grid_short: &'static str,
    pub menu_help: &'static str,
    pub about: &'static str,
    pub source: &'static str,
    pub target: &'static str,
    pub convert: &'static str,
    pub search: &'static str,
    pub search_hint: &'static str,
    pub playlists: &'static str,
    pub all_tracks: &'static str,
    pub no_library: &'static str,
    pub no_library_hint: &'static str,
    pub no_results: &'static str,
    pub no_results_hint: &'static str,
    pub detail: &'static str,
    pub no_selection: &'static str,
    pub color: &'static str,
    pub key: &'static str,
    pub bpm: &'static str,
    pub duration: &'static str,
    pub cue_points: &'static str,
    pub compatible_only: &'static str,
    pub compatible_hint: &'static str,
    pub compatible_needs_selection: &'static str,
    pub col_cover: &'static str,
    pub col_title: &'static str,
    pub col_artist: &'static str,
    pub col_bpm: &'static str,
    pub col_key: &'static str,
    pub col_duration: &'static str,
    pub status_ready: &'static str,
    pub status_export_todo: &'static str,
    pub status_busy: &'static str,
    pub unknown_playlist: &'static str,
    pub about_title: &'static str,
    pub about_body: &'static str,
    pub about_todo: &'static str,
}

pub const ES: Texts = Texts {
    app_title: "DJ Library Manager",
    menu_file: "Archivo",
    detect_libraries: "Detectar librerías...",
    detect_title: "Detectar librerías",
    detect_hint: "Librerías encontradas en este equipo. Se abren en modo de solo lectura: el original nunca se modifica.",
    detect_scanning: "Buscando librerías de Rekordbox y Traktor...",
    detect_none: "No se han encontrado librerías compatibles.",
    detect_none_hint: "Comprueba que Rekordbox o Traktor estén instalados y se hayan abierto al menos una vez. En Rekordbox 6, exporta la colección a XML (rekordbox.xml): master.db está cifrada.",
    detect_rescan: "Buscar de nuevo",
    detect_demo_note: "Demostración con datos de ejemplo: no se lee tu librería real.",
    demo_tag: "Datos de ejemplo",
    count_at_open: "El recuento se calcula al abrir la librería.",
    program_running_note: "Programa abierto: ciérralo para poder abrir esta librería.",
    rekordbox_xml_note: "Se lee la exportación XML (rekordbox.xml), que puede no incluir los últimos cambios. master.db está cifrada y no se toca.",
    unknown_library: "La librería ya no está disponible. Vuelve a buscar.",
    unsupported_program: "Ese programa todavía no tiene lector de solo lectura.",
    open: "Abrir",
    cancel: "Cancelar",
    status_detecting: "Buscando librerías...",
    status_worker_lost: "La operación se interrumpió sin terminar.",
    export: "Exportar...",
    quit: "Salir",
    menu_view: "Ver",
    view_list: "Lista",
    view_grid: "Rejilla con portadas",
    view_grid_short: "Rejilla",
    menu_help: "Ayuda",
    about: "Acerca de DJ Library Manager",
    source: "Origen",
    target: "Destino",
    convert: "Convertir",
    search: "Buscar",
    search_hint: "Título, artista o álbum",
    playlists: "Playlists",
    all_tracks: "Toda la librería",
    no_library: "Sin librería cargada",
    no_library_hint: "Usa Archivo > Detectar librerías... o pulsa Ctrl+O.",
    no_results: "Sin resultados",
    no_results_hint: "Prueba con otra búsqueda o desactiva el filtro de armonía.",
    detail: "Detalle",
    no_selection: "Selecciona una pista para ver su detalle.",
    color: "Color",
    key: "Clave",
    bpm: "BPM",
    duration: "Duración",
    cue_points: "Puntos de cue",
    compatible_only: "Solo claves compatibles",
    compatible_hint: "Compatible: misma posición o +-1 con la misma letra.",
    compatible_needs_selection: "Selecciona una pista con clave para filtrar por armonía.",
    col_cover: "Port.",
    col_title: "Título",
    col_artist: "Artista",
    col_bpm: "BPM",
    col_key: "Clave",
    col_duration: "Dur.",
    status_ready: "Listo. Abre una librería para empezar.",
    status_export_todo: "Exportación pendiente: falta el escritor de formatos (TODO).",
    status_busy: "Hay una carga en curso. Espera a que termine.",
    unknown_playlist: "Playlist desconocida",
    about_title: "Acerca de",
    about_body: "DJ Library Manager\nGestor de librerías DJ sin conexión.\n\nInterfaz: egui/eframe 0.29.",
    about_todo: "Pendiente: escritores y conversión para Rekordbox, Serato DJ, Traktor, Virtual DJ y Engine DJ; portadas reales; exportación.",
};

/// Número con separador de miles: `50000` -> `50.000`.
pub fn number(value: usize) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(digit);
    }
    grouped.chars().rev().collect()
}

/// `1 pista` / `1.250 pistas`.
pub fn track_count(count: usize) -> String {
    if count == 1 {
        "1 pista".to_string()
    } else {
        format!("{} pistas", number(count))
    }
}

/// `1 playlist` / `8 playlists`.
pub fn playlist_count(count: usize) -> String {
    if count == 1 {
        "1 playlist".to_string()
    } else {
        format!("{} playlists", number(count))
    }
}

/// `320 de 50.000 pistas` cuando hay filtros activos.
pub fn filtered_count(visible: usize, total: usize) -> String {
    format!("{} de {}", number(visible), track_count(total))
}

/// Resumen de una librería detectada: `50.000 pistas y 8 playlists`.
pub fn detected_summary(tracks: usize, playlists: usize) -> String {
    format!("{} y {}", track_count(tracks), playlist_count(playlists))
}

/// Igual que `detected_summary`, pero admite recuentos aún desconocidos.
pub fn detected_summary_optional(tracks: Option<usize>, playlists: Option<usize>) -> String {
    match (tracks, playlists) {
        (Some(tracks), Some(playlists)) => detected_summary(tracks, playlists),
        (Some(tracks), None) => track_count(tracks),
        (None, Some(playlists)) => playlist_count(playlists),
        (None, None) => ES.count_at_open.to_string(),
    }
}

/// Error al intentar abrir con el programa de DJ todavía en marcha.
pub fn open_program_running(program: &str) -> String {
    format!("Cierra {program} antes de abrir la librería.")
}

/// `1 librería detectada` / `2 librerías detectadas`.
pub fn detected(count: usize) -> String {
    if count == 1 {
        "1 librería detectada.".to_string()
    } else {
        format!("{} librerías detectadas.", number(count))
    }
}

/// Mensaje de la barra de estado al abrir una candidata.
pub fn opening(name: &str) -> String {
    format!("Abriendo {name}...")
}

/// Mensaje al terminar de cargar una librería.
pub fn loaded(name: &str, tracks: usize, playlists: usize) -> String {
    format!(
        "{name}: {} y {} cargadas.",
        track_count(tracks),
        playlist_count(playlists)
    )
}

/// Error devuelto por el proveedor al abrir una librería.
pub fn open_failed(message: &str) -> String {
    format!("No se pudo abrir la librería: {message}")
}

/// Mensaje del aviso de conversión pendiente.
pub fn conversion_pending(target: &str) -> String {
    format!("Conversión pendiente: falta el escritor de {target} (TODO).")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_grouped_in_thousands() {
        assert_eq!(number(0), "0");
        assert_eq!(number(999), "999");
        assert_eq!(number(1_000), "1.000");
        assert_eq!(number(50_000), "50.000");
        assert_eq!(number(1_234_567), "1.234.567");
    }

    #[test]
    fn singular_and_plural_counts() {
        assert_eq!(track_count(1), "1 pista");
        assert_eq!(track_count(2), "2 pistas");
        assert_eq!(filtered_count(320, 50_000), "320 de 50.000 pistas");
    }

    #[test]
    fn detection_texts_format_counts() {
        assert_eq!(detected_summary(50_000, 8), "50.000 pistas y 8 playlists");
        assert_eq!(detected(1), "1 librería detectada.");
        assert_eq!(detected(2), "2 librerías detectadas.");
        assert_eq!(
            loaded("Rekordbox (ejemplo)", 1, 1),
            "Rekordbox (ejemplo): 1 pista y 1 playlist cargadas."
        );
    }
}
