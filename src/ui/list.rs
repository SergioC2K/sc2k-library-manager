//! Vista de lista virtualizada con `TableBuilder`.

use egui::{Align, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};

use crate::covers::CoverCache;
use crate::i18n::ES;
use crate::model::{Library, TrackId};
use crate::theme;
use crate::ui::chips;

const ROW_HEIGHT: f32 = 40.0;
const COVER_SIZE: f32 = 28.0;

/// Dibuja la tabla de pistas y devuelve la pista clicada, si hubo.
pub fn show(
    ui: &mut Ui,
    library: &Library,
    visible: &[TrackId],
    selected: Option<TrackId>,
    covers: &mut CoverCache,
) -> Option<TrackId> {
    let mut clicked = None;

    TableBuilder::new(ui)
        // Sin esto el `TableBuilder` usa `Sense::hover()` y la fila no responde.
        .sense(Sense::click())
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(Align::Center))
        .column(Column::exact(26.0))
        .column(Column::exact(38.0))
        .column(Column::remainder().at_least(150.0).clip(true))
        .column(Column::remainder().at_least(110.0).clip(true))
        .column(Column::exact(62.0))
        .column(Column::exact(52.0))
        .column(Column::exact(62.0))
        .min_scrolled_height(0.0)
        .header(22.0, |mut header| {
            header.col(|_ui| {});
            header.col(|ui| {
                ui.weak(ES.col_cover);
            });
            header.col(|ui| {
                ui.strong(ES.col_title);
            });
            header.col(|ui| {
                ui.strong(ES.col_artist);
            });
            header.col(|ui| {
                ui.strong(ES.col_bpm);
            });
            header.col(|ui| {
                ui.strong(ES.col_key);
            });
            header.col(|ui| {
                ui.strong(ES.col_duration);
            });
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, visible.len(), |mut row| {
                let index = row.index();
                let Some(&track_id) = visible.get(index) else {
                    return;
                };
                let Some(track) = library.tracks.get(track_id) else {
                    return;
                };

                row.set_selected(selected == Some(track_id));

                row.col(|ui| {
                    chips::color_dot(ui, track.color, 10.0);
                });
                row.col(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(COVER_SIZE), Sense::hover());
                    covers.draw(ui, track, rect);
                });
                row.col(|ui| {
                    ui.add(
                        egui::Label::new(RichText::new(&track.title).strong())
                            .truncate()
                            .selectable(false),
                    );
                });
                row.col(|ui| {
                    ui.add(
                        egui::Label::new(RichText::new(&track.artist).color(theme::TEXT_DIM))
                            .truncate()
                            .selectable(false),
                    );
                });
                row.col(|ui| {
                    ui.add(egui::Label::new(format!("{:.1}", track.bpm)).selectable(false));
                });
                row.col(|ui| {
                    if let Some(key) = track.key {
                        chips::key_chip(ui, key);
                    } else {
                        ui.weak("-");
                    }
                });
                row.col(|ui| {
                    ui.add(egui::Label::new(track.duration_label()).selectable(false));
                });

                if row.response().clicked() {
                    clicked = Some(track_id);
                }
            });
        });

    clicked
}
