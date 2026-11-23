//! Panel izquierdo: entrada "Toda la libreria" y arbol de carpetas/playlists.

use egui::{CollapsingHeader, RichText, Ui};

use crate::i18n::{self, ES};
use crate::model::{Library, PlaylistNode, Selection};
use crate::theme;

/// Devuelve `true` si el usuario cambio la seleccion.
pub fn show(ui: &mut Ui, library: &Library, selection: &mut Selection) -> bool {
    ui.label(RichText::new(&library.name).strong());
    ui.label(
        RichText::new(library.source.label())
            .small()
            .color(theme::TEXT_DIM),
    );
    ui.add_space(4.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(RichText::new(ES.playlists).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(i18n::number(library.playlist_count()))
                    .small()
                    .color(theme::TEXT_DIM),
            );
        });
    });
    ui.add_space(2.0);

    let mut changed = false;
    let all_label = format!(
        "{}  ({})",
        ES.all_tracks,
        i18n::number(library.tracks.len())
    );
    if ui
        .selectable_label(*selection == Selection::AllTracks, all_label)
        .clicked()
        && *selection != Selection::AllTracks
    {
        *selection = Selection::AllTracks;
        changed = true;
    }

    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for node in &library.playlists {
                if show_node(ui, node, selection) {
                    changed = true;
                }
            }
        });

    changed
}

fn show_node(ui: &mut Ui, node: &PlaylistNode, selection: &mut Selection) -> bool {
    let mut changed = false;
    match node {
        PlaylistNode::Folder { name, children } => {
            CollapsingHeader::new(RichText::new(name).color(theme::TEXT_DIM))
                .id_salt(("carpeta", name))
                .default_open(true)
                .show(ui, |ui| {
                    for child in children {
                        if show_node(ui, child, selection) {
                            changed = true;
                        }
                    }
                });
        }
        PlaylistNode::Playlist {
            id,
            name,
            track_ids,
        } => {
            let selected = *selection == Selection::Playlist(*id);
            let label = format!("{name}  ({})", i18n::number(track_ids.len()));
            if ui.selectable_label(selected, label).clicked() && !selected {
                *selection = Selection::Playlist(*id);
                changed = true;
            }
        }
    }
    changed
}
