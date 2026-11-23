//! Piezas de dibujo compartidas: chip de clave Camelot y punto de color.

use egui::{Align2, Color32, FontId, Response, Sense, Ui, Vec2};

use crate::harmony::CamelotKey;
use crate::model::TrackColor;

/// Chip pequeno de clave para listas y rejillas.
pub fn key_chip(ui: &mut Ui, key: CamelotKey) -> Response {
    key_chip_sized(ui, key, Vec2::new(34.0, 18.0), 11.0)
}

/// Chip de clave con tamano y fuente configurables (panel de detalle).
pub fn key_chip_sized(ui: &mut Ui, key: CamelotKey, size: Vec2, font_size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0, key.color());
    // Texto negro sobre el chip: el color de clave se elige con luminancia alta.
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        key.label(),
        FontId::proportional(font_size),
        Color32::BLACK,
    );
    response
}

/// Punto de color de pista; sin color dibuja un anillo tenue.
pub fn color_dot(ui: &mut Ui, color: TrackColor, diameter: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(diameter), Sense::hover());
    let painter = ui.painter();
    match color.swatch() {
        Some(fill) => {
            painter.circle_filled(rect.center(), diameter * 0.5, fill);
        }
        None => {
            painter.circle_stroke(
                rect.center(),
                (diameter * 0.5 - 1.0).max(1.0),
                egui::Stroke::new(1.0, Color32::from_gray(96)),
            );
        }
    }
    response
}
