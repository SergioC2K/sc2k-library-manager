//! Panel derecho: detalle de la pista seleccionada y filtro de armonia.

use egui::{RichText, Ui, Vec2};

use crate::covers::CoverCache;
use crate::i18n::ES;
use crate::model::{Library, TrackId};
use crate::theme;
use crate::ui::chips;

pub fn show(
    ui: &mut Ui,
    library: Option<&Library>,
    selected: Option<TrackId>,
    compatible_only: &mut bool,
    covers: &mut CoverCache,
) {
    ui.label(RichText::new(ES.detail).strong());
    ui.separator();

    let track = library
        .zip(selected)
        .and_then(|(library, id)| library.tracks.get(id));

    let Some(track) = track else {
        show_no_selection(ui, compatible_only);
        return;
    };

    let side = 180.0_f32.min(ui.available_width());
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::hover());
        covers.draw(ui, track, rect);
    });
    ui.add_space(10.0);
    ui.label(RichText::new(&track.title).heading());
    ui.label(RichText::new(&track.artist).color(theme::TEXT_DIM));
    ui.label(RichText::new(&track.album).italics().color(theme::TEXT_DIM));

    ui.add_space(10.0);
    egui::Grid::new("detalle-pista")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            ui.label(RichText::new(ES.color).color(theme::TEXT_DIM));
            ui.horizontal(|ui| {
                chips::color_dot(ui, track.color, 12.0);
                ui.label(track.color.label());
            });
            ui.end_row();

            ui.label(RichText::new(ES.key).color(theme::TEXT_DIM));
            if let Some(key) = track.key {
                ui.horizontal(|ui| {
                    chips::key_chip_sized(ui, key, Vec2::new(40.0, 20.0), 12.0);
                    ui.label(
                        RichText::new(key.mode.label())
                            .small()
                            .color(theme::TEXT_DIM),
                    );
                });
            } else {
                ui.weak("-");
            }
            ui.end_row();

            ui.label(RichText::new(ES.bpm).color(theme::TEXT_DIM));
            ui.label(format!("{:.1}", track.bpm));
            ui.end_row();

            ui.label(RichText::new(ES.duration).color(theme::TEXT_DIM));
            ui.label(track.duration_label());
            ui.end_row();

            ui.label(RichText::new(ES.cue_points).color(theme::TEXT_DIM));
            ui.label(track.cue_points.to_string());
            ui.end_row();
        });

    ui.add_space(12.0);
    let has_key = track.key.is_some();
    let checkbox = ui.add_enabled(
        has_key,
        egui::Checkbox::new(compatible_only, ES.compatible_only),
    );
    if has_key {
        ui.label(
            RichText::new(ES.compatible_hint)
                .small()
                .color(theme::TEXT_DIM),
        );
    } else {
        checkbox.on_hover_text(ES.compatible_needs_selection);
        ui.label(
            RichText::new(ES.compatible_needs_selection)
                .small()
                .color(theme::TEXT_DIM),
        );
    }
}

fn show_no_selection(ui: &mut Ui, compatible_only: &mut bool) {
    ui.add_space(16.0);
    ui.label(RichText::new(ES.no_selection).color(theme::TEXT_DIM));
    ui.add_space(12.0);
    let checkbox = ui.add_enabled(
        false,
        egui::Checkbox::new(compatible_only, ES.compatible_only),
    );
    checkbox.on_hover_text(ES.compatible_needs_selection);
    ui.add_space(4.0);
    ui.label(
        RichText::new(ES.compatible_needs_selection)
            .small()
            .color(theme::TEXT_DIM),
    );
}
