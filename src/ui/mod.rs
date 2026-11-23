//! Piezas de interfaz de la pantalla principal.

pub mod chips;
pub mod detail;
pub mod detect;
pub mod grid;
pub mod list;
pub mod playlists;
pub mod status;
pub mod top_bar;

/// Conmutador de vista del panel central.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    List,
    Grid,
}
