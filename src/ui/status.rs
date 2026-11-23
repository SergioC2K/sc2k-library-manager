//! Barra de estado inferior: mensajes de progreso y avisos.

use egui::{RichText, Ui};

use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Progress,
    Warning,
}

pub struct Status {
    pub message: String,
    pub kind: StatusKind,
    pub progress: Option<f32>,
}

impl Status {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: StatusKind::Info,
            progress: None,
        }
    }

    pub fn progress(message: impl Into<String>, fraction: f32) -> Self {
        Self {
            message: message.into(),
            kind: StatusKind::Progress,
            progress: Some(fraction.clamp(0.0, 1.0)),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            kind: StatusKind::Warning,
            progress: None,
        }
    }
}

pub fn show(ui: &mut Ui, status: &Status) {
    ui.horizontal(|ui| {
        let color = match status.kind {
            StatusKind::Info | StatusKind::Progress => theme::TEXT_DIM,
            StatusKind::Warning => theme::WARNING,
        };
        ui.add(egui::Label::new(RichText::new(&status.message).color(color)).truncate());

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(fraction) = status.progress {
                ui.add(
                    egui::ProgressBar::new(fraction)
                        .desired_width(180.0)
                        .show_percentage()
                        .fill(theme::ACCENT),
                );
            }
        });
    });
}
