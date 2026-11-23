//! Barra superior: menu, selector de origen/destino, boton Convertir y buscador.

use egui::{Color32, ComboBox, Layout, RichText, Ui};

use crate::i18n::ES;
use crate::model::DeckProgram;
use crate::theme;
use crate::ui::ViewMode;

/// Acciones que la barra superior delega en `App`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopBarAction {
    DetectLibraries,
    Export,
    Quit,
    Convert,
    About,
}

pub struct TopBarState<'a> {
    pub source: &'a mut DeckProgram,
    pub target: &'a mut DeckProgram,
    pub search: &'a mut String,
    pub view: &'a mut ViewMode,
    pub library_loaded: bool,
    pub focus_search: &'a mut bool,
}

pub fn show(ui: &mut Ui, state: &mut TopBarState<'_>) -> Vec<TopBarAction> {
    let mut actions = Vec::new();

    egui::menu::bar(ui, |ui| {
        ui.menu_button(ES.menu_file, |ui| {
            if ui
                .add(egui::Button::new(ES.detect_libraries).shortcut_text("Ctrl+O"))
                .clicked()
            {
                actions.push(TopBarAction::DetectLibraries);
                ui.close_menu();
            }
            if ui.add(egui::Button::new(ES.export)).clicked() {
                actions.push(TopBarAction::Export);
                ui.close_menu();
            }
            ui.separator();
            if ui.add(egui::Button::new(ES.quit)).clicked() {
                actions.push(TopBarAction::Quit);
                ui.close_menu();
            }
        });

        ui.menu_button(ES.menu_view, |ui| {
            if ui
                .selectable_label(*state.view == ViewMode::List, ES.view_list)
                .clicked()
            {
                *state.view = ViewMode::List;
                ui.close_menu();
            }
            if ui
                .selectable_label(*state.view == ViewMode::Grid, ES.view_grid)
                .clicked()
            {
                *state.view = ViewMode::Grid;
                ui.close_menu();
            }
        });

        ui.menu_button(ES.menu_help, |ui| {
            if ui.button(ES.about).clicked() {
                actions.push(TopBarAction::About);
                ui.close_menu();
            }
        });
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(ES.source).color(theme::TEXT_DIM));
        ComboBox::from_id_salt("programa-origen")
            .width(126.0)
            .selected_text(state.source.label())
            .show_ui(ui, |ui| {
                for program in DeckProgram::ALL {
                    ui.selectable_value(&mut *state.source, program, program.label());
                }
            });

        ui.add_space(10.0);
        ui.label(RichText::new(ES.target).color(theme::TEXT_DIM));
        ComboBox::from_id_salt("programa-destino")
            .width(126.0)
            .selected_text(state.target.label())
            .show_ui(ui, |ui| {
                for program in DeckProgram::ALL {
                    ui.selectable_value(&mut *state.target, program, program.label());
                }
            });

        let can_convert = state.library_loaded && *state.source != *state.target;
        let label = if can_convert {
            RichText::new(ES.convert).strong().color(Color32::BLACK)
        } else {
            RichText::new(ES.convert).color(theme::TEXT_DIM)
        };
        let fill = if can_convert {
            theme::ACCENT
        } else {
            theme::PANEL_ALT
        };
        if ui
            .add_enabled(can_convert, egui::Button::new(label).fill(fill))
            .clicked()
        {
            actions.push(TopBarAction::Convert);
        }

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut *state.search)
                    .hint_text(ES.search_hint)
                    .desired_width(260.0),
            );
            if *state.focus_search {
                response.request_focus();
                *state.focus_search = false;
            }
            ui.label(RichText::new(ES.search).color(theme::TEXT_DIM));
        });
    });

    actions
}
