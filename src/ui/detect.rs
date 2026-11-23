//! Diálogo de detección de librerías instaladas.
//!
//! La ventana no conoce formatos ni rutas: solo presenta las candidatas que
//! devuelve `LibraryProvider` y pide abrir una de ellas.

use egui::{Align2, Context, RichText};

use crate::detection::{DetectedLibrary, DetectedLibraryId};
use crate::i18n::{self, ES};
use crate::theme;

/// Datos que el diálogo lee y modifica.
pub struct DetectState<'a> {
    pub candidates: &'a [DetectedLibrary],
    pub choice: &'a mut Option<DetectedLibraryId>,
    pub detecting: bool,
    pub is_demo: bool,
    pub busy: bool,
    pub open: &'a mut bool,
}

/// Lo que el usuario pidió hacer en este frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectAction {
    Open(DetectedLibraryId),
    Rescan,
}

pub fn show(ctx: &Context, state: &mut DetectState<'_>) -> Option<DetectAction> {
    let candidates = state.candidates;
    let detecting = state.detecting;
    let is_demo = state.is_demo;
    let busy = state.busy;
    let mut choice = *state.choice;
    let mut open = *state.open;
    let mut close = false;
    let mut action = None;

    egui::Window::new(ES.detect_title)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(560.0)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(RichText::new(ES.detect_hint).color(theme::TEXT_DIM));
            if is_demo {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("{} · {}", ES.demo_tag, ES.detect_demo_note))
                        .small()
                        .color(theme::WARNING),
                );
            }
            ui.add_space(10.0);

            if detecting {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(ES.detect_scanning);
                });
            } else if candidates.is_empty() {
                ui.label(RichText::new(ES.detect_none).strong());
                ui.add_space(2.0);
                ui.label(RichText::new(ES.detect_none_hint).color(theme::TEXT_DIM));
            } else {
                for candidate in candidates {
                    let selected = choice == Some(candidate.id);
                    let mut text = format!(
                        "{}  ·  {}\n{}\n{}",
                        candidate.name,
                        candidate.program.label(),
                        candidate.location,
                        i18n::detected_summary_optional(
                            candidate.track_count,
                            candidate.playlist_count
                        )
                    );
                    if let Some(note) = &candidate.note {
                        text.push('\n');
                        text.push_str(note);
                    }
                    if candidate.program_running {
                        text.push('\n');
                        text.push_str(ES.program_running_note);
                    }

                    let mut label = RichText::new(text);
                    if candidate.program_running {
                        label = label.color(theme::WARNING);
                    }
                    let response = ui.add(egui::SelectableLabel::new(selected, label));
                    if response.clicked() {
                        choice = Some(candidate.id);
                    }
                    if response.double_clicked() && !candidate.program_running {
                        action = Some(DetectAction::Open(candidate.id));
                    }
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                let selected_running = choice
                    .and_then(|id| {
                        candidates
                            .iter()
                            .find(|candidate| candidate.id == id)
                            .map(|candidate| candidate.program_running)
                    })
                    .unwrap_or(false);
                let can_open = !detecting && !busy && choice.is_some() && !selected_running;
                if ui
                    .add_enabled(can_open, egui::Button::new(ES.open))
                    .clicked()
                {
                    action = choice.map(DetectAction::Open);
                }
                if ui
                    .add_enabled(!detecting, egui::Button::new(ES.detect_rescan))
                    .clicked()
                {
                    action = Some(DetectAction::Rescan);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(ES.cancel).clicked() {
                        close = true;
                    }
                });
            });
        });

    if close {
        open = false;
    }
    *state.open = open;
    *state.choice = choice;
    action
}
