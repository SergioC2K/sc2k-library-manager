//! Tema oscuro y colores compartidos por la pantalla.

use egui::{Color32, FontFamily, FontId, Frame, Margin, Stroke, TextStyle};

pub const BG: Color32 = Color32::from_rgb(16, 17, 21);
pub const PANEL: Color32 = Color32::from_rgb(23, 25, 31);
pub const PANEL_ALT: Color32 = Color32::from_rgb(31, 34, 41);
pub const BORDER: Color32 = Color32::from_rgb(46, 50, 60);
pub const TEXT_DIM: Color32 = Color32::from_rgb(156, 162, 176);
pub const TEXT: Color32 = Color32::from_rgb(226, 229, 235);
pub const ACCENT: Color32 = Color32::from_rgb(76, 158, 232);
pub const SELECTED_BG: Color32 = Color32::from_rgb(38, 66, 102);
pub const WARNING: Color32 = Color32::from_rgb(232, 184, 92);

/// Marco de panel con relleno y margen interior.
pub fn frame(margin: f32) -> Frame {
    Frame::none()
        .fill(PANEL)
        .inner_margin(Margin::same(margin))
        .stroke(Stroke::new(1.0, BORDER))
}

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(12, 13, 17);
    visuals.faint_bg_color = PANEL_ALT;
    visuals.menu_rounding = 6.0.into();
    visuals.window_rounding = 8.0.into();
    visuals.selection.bg_fill = SELECTED_BG;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(214, 218, 226));
    visuals.widgets.inactive.bg_fill = PANEL_ALT;
    visuals.widgets.inactive.weak_bg_fill = PANEL_ALT;
    visuals.widgets.inactive.rounding = 4.0.into();
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 44, 54);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(40, 44, 54);
    visuals.widgets.hovered.rounding = 4.0.into();
    visuals.widgets.active.bg_fill = SELECTED_BG;
    visuals.widgets.active.weak_bg_fill = SELECTED_BG;
    visuals.widgets.active.rounding = 4.0.into();
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = 24.0;
    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(19.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(12.5, FontFamily::Monospace),
        ),
    ]
    .into();
    ctx.set_style(style);
}
