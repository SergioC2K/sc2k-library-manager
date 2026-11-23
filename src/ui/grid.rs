//! Vista de rejilla: tarjetas de 120 px, virtualizadas por filas.

use egui::{Align2, Color32, FontId, Rect, Response, Sense, Ui, Vec2};

use crate::covers::CoverCache;
use crate::model::{Library, Track, TrackId};
use crate::theme;

const CARD_WIDTH: f32 = 132.0;
const CARD_HEIGHT: f32 = 188.0;
const COVER_SIZE: f32 = 120.0;
const PADDING: f32 = 6.0;
const GAP: f32 = 10.0;

/// Dibuja la rejilla y devuelve la pista clicada, si hubo.
pub fn show(
    ui: &mut Ui,
    library: &Library,
    visible: &[TrackId],
    selected: Option<TrackId>,
    covers: &mut CoverCache,
) -> Option<TrackId> {
    let mut clicked = None;
    let available = ui.available_width();
    let columns = (((available + GAP) / (CARD_WIDTH + GAP)).floor() as usize).max(1);
    let rows = visible.len().div_ceil(columns);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, CARD_HEIGHT + GAP, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    for column in 0..columns {
                        let index = row * columns + column;
                        let Some(&track_id) = visible.get(index) else {
                            break;
                        };
                        let Some(track) = library.tracks.get(track_id) else {
                            continue;
                        };

                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(CARD_WIDTH, CARD_HEIGHT),
                            Sense::click(),
                        );
                        let response =
                            response.on_hover_text(format!("{}\n{}", track.title, track.artist));
                        draw_card(
                            ui,
                            covers,
                            rect,
                            &response,
                            track,
                            selected == Some(track_id),
                        );
                        if response.clicked() {
                            clicked = Some(track_id);
                        }
                    }
                });
            }
        });

    clicked
}

fn draw_card(
    ui: &Ui,
    covers: &mut CoverCache,
    rect: Rect,
    response: &Response,
    track: &Track,
    selected: bool,
) {
    let background = if selected {
        theme::SELECTED_BG
    } else if response.hovered() {
        theme::PANEL_ALT
    } else {
        theme::PANEL
    };

    let painter = ui.painter();
    painter.rect_filled(rect, 6.0, background);
    if selected {
        painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, theme::ACCENT));
    } else if response.hovered() {
        painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, theme::BORDER));
    }

    let cover = Rect::from_min_size(rect.min + Vec2::splat(PADDING), Vec2::splat(COVER_SIZE));
    covers.draw(ui, track, cover);

    if let Some(swatch) = track.color.swatch() {
        let center = egui::pos2(cover.right() - 9.0, cover.top() + 9.0);
        painter.circle_filled(center, 4.5, swatch);
        painter.circle_stroke(
            center,
            4.5,
            egui::Stroke::new(1.0, Color32::from_black_alpha(120)),
        );
    }

    if let Some(key) = track.key {
        let chip = Rect::from_min_size(
            egui::pos2(cover.right() - 4.0 - 34.0, cover.bottom() - 4.0 - 18.0),
            Vec2::new(34.0, 18.0),
        );
        painter.rect_filled(chip, 3.0, key.color());
        painter.text(
            chip.center(),
            Align2::CENTER_CENTER,
            key.label(),
            FontId::proportional(11.0),
            Color32::BLACK,
        );
    }

    let title_position = egui::pos2(cover.left() + 2.0, cover.bottom() + 8.0);
    painter.text(
        title_position,
        Align2::LEFT_TOP,
        truncate(&track.title, 18),
        FontId::proportional(12.5),
        theme::TEXT,
    );
    painter.text(
        title_position + Vec2::new(0.0, 17.0),
        Align2::LEFT_TOP,
        truncate(&track.artist, 19),
        FontId::proportional(11.5),
        theme::TEXT_DIM,
    );
    painter.text(
        egui::pos2(rect.left() + 8.0, rect.bottom() - 7.0),
        Align2::LEFT_BOTTOM,
        format!("{:.1} BPM", track.bpm),
        FontId::proportional(10.5),
        theme::TEXT_DIM,
    );
    painter.text(
        egui::pos2(rect.right() - 8.0, rect.bottom() - 7.0),
        Align2::RIGHT_BOTTOM,
        track.duration_label(),
        FontId::proportional(10.5),
        theme::TEXT_DIM,
    );
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let mut truncated: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncate_keeps_short_text_and_shortens_long_text() {
        assert_eq!(truncate("Corto", 18), "Corto");
        let shortened = truncate("Un titulo bastante mas largo que el ancho", 18);
        assert_eq!(shortened.chars().count(), 18);
        assert!(shortened.ends_with("..."));
    }
}
