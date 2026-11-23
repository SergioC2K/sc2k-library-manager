//! Estado de la aplicación y composición de paneles.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;

use egui::{Context, Key, RichText};

use crate::covers::CoverCache;
use crate::detection::{DetectedLibrary, DetectedLibraryId, LibraryProvider};
use crate::harmony::CamelotKey;
use crate::i18n::{self, ES};
use crate::model::{DeckProgram, Library, Selection, TrackId};
use crate::theme;
use crate::ui::{self, detect::DetectAction, status::Status, top_bar::TopBarAction, ViewMode};

enum JobMessage {
    Progress(f32),
    Loaded(Box<Library>),
    Failed(String),
}

pub struct App {
    provider: Arc<dyn LibraryProvider>,
    library: Option<Library>,
    /// Copia en minúsculas de título+artista+álbum para buscar sin asignar por pista.
    search_index: Vec<String>,
    selection: Selection,
    last_selection: Selection,
    selected_track: Option<TrackId>,
    harmony_anchor: Option<CamelotKey>,
    visible: Vec<TrackId>,
    filters_dirty: bool,
    view: ViewMode,
    search: String,
    last_search: String,
    source: DeckProgram,
    target: DeckProgram,
    compatible_only: bool,
    status: Status,
    covers: CoverCache,
    job: Option<Receiver<JobMessage>>,
    job_label: String,
    detecting: bool,
    detect_receiver: Option<Receiver<Vec<DetectedLibrary>>>,
    detect_open: bool,
    candidates: Vec<DetectedLibrary>,
    detected_choice: Option<DetectedLibraryId>,
    about_open: bool,
    focus_search: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, provider: Arc<dyn LibraryProvider>) -> Self {
        theme::apply(&cc.egui_ctx);
        Self {
            provider,
            library: None,
            search_index: Vec::new(),
            selection: Selection::AllTracks,
            last_selection: Selection::AllTracks,
            selected_track: None,
            harmony_anchor: None,
            visible: Vec::new(),
            filters_dirty: false,
            view: ViewMode::List,
            search: String::new(),
            last_search: String::new(),
            source: DeckProgram::Rekordbox,
            target: DeckProgram::Serato,
            compatible_only: false,
            status: Status::info(ES.status_ready),
            covers: CoverCache::new(&cc.egui_ctx),
            job: None,
            job_label: String::new(),
            detecting: false,
            detect_receiver: None,
            detect_open: false,
            candidates: Vec::new(),
            detected_choice: None,
            about_open: false,
            focus_search: false,
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let (open, find, escape) = ctx.input(|input| {
            (
                input.modifiers.command && input.key_pressed(Key::O),
                input.modifiers.command && input.key_pressed(Key::F),
                input.key_pressed(Key::Escape),
            )
        });

        if open {
            self.open_detection(ctx);
        }
        if find {
            self.focus_search = true;
        }
        if escape && self.detect_open {
            self.detect_open = false;
        }

        // Con un diálogo abierto o un campo de texto enfocado, las flechas
        // pertenecen a ese widget, no a la lista de pistas.
        if self.detect_open || ctx.memory(|memory| memory.focused().is_some()) {
            return;
        }
        let (up, down) = ctx.input(|input| {
            (
                input.key_pressed(Key::ArrowUp),
                input.key_pressed(Key::ArrowDown),
            )
        });
        if up {
            self.step_selection(-1);
        } else if down {
            self.step_selection(1);
        }
    }

    fn step_selection(&mut self, delta: isize) {
        if self.visible.is_empty() {
            return;
        }
        let last = self.visible.len() - 1;
        let index = match self
            .selected_track
            .and_then(|id| self.visible.iter().position(|candidate| *candidate == id))
        {
            Some(position) => (position as isize + delta).clamp(0, last as isize) as usize,
            None if delta >= 0 => 0,
            None => last,
        };
        self.select(self.visible[index]);
    }

    fn select(&mut self, track_id: TrackId) {
        self.selected_track = Some(track_id);
        self.harmony_anchor = self
            .library
            .as_ref()
            .and_then(|library| library.tracks.get(track_id))
            .and_then(|track| track.key);
        if self.compatible_only {
            // Una pista sin clave no puede anclar el filtro de armonia.
            self.compatible_only = self.harmony_anchor.is_some();
            self.filters_dirty = true;
        }
    }

    fn open_detection(&mut self, ctx: &Context) {
        if self.detect_receiver.is_some() {
            self.detect_open = true;
            return;
        }
        if self.candidates.is_empty() {
            self.start_detection(ctx);
        } else {
            self.detect_open = true;
        }
    }

    /// Busca librerías en un hilo de fondo; la UI se repinta al terminar.
    fn start_detection(&mut self, ctx: &Context) {
        if self.detect_receiver.is_some() {
            return;
        }

        self.detect_open = true;
        self.detecting = true;
        self.candidates.clear();
        self.detected_choice = None;
        self.status = Status::info(ES.status_detecting);

        let (sender, receiver) = mpsc::channel();
        let provider = Arc::clone(&self.provider);
        let worker_ctx = ctx.clone();
        thread::spawn(move || {
            let found = provider.detect();
            let _ = sender.send(found);
            worker_ctx.request_repaint();
        });
        self.detect_receiver = Some(receiver);
    }

    fn poll_detection(&mut self) {
        let mut found = None;
        let mut lost = false;
        if let Some(receiver) = &self.detect_receiver {
            match receiver.try_recv() {
                Ok(candidates) => found = Some(candidates),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => lost = true,
            }
        }

        if let Some(candidates) = found {
            self.detect_receiver = None;
            self.detecting = false;
            self.status = Status::info(i18n::detected(candidates.len()));
            if self.detected_choice.is_none() {
                self.detected_choice = candidates.first().map(|candidate| candidate.id);
            }
            self.candidates = candidates;
        } else if lost {
            self.detect_receiver = None;
            self.detecting = false;
            self.status = Status::warning(ES.status_worker_lost);
        }
    }

    /// Abre la candidata elegida en un hilo de fondo, informando del progreso.
    fn start_open(&mut self, ctx: &Context, id: DetectedLibraryId, name: &str) {
        if self.job.is_some() {
            self.status = Status::warning(ES.status_busy);
            return;
        }

        let (sender, receiver) = mpsc::channel();
        let provider = Arc::clone(&self.provider);
        let worker_ctx = ctx.clone();
        thread::spawn(move || {
            let result = provider.open(id, &mut |fraction| {
                let _ = sender.send(JobMessage::Progress(fraction));
                worker_ctx.request_repaint();
            });
            let message = match result {
                Ok(library) => JobMessage::Loaded(Box::new(library)),
                Err(error) => JobMessage::Failed(error),
            };
            let _ = sender.send(message);
            worker_ctx.request_repaint();
        });

        self.job = Some(receiver);
        self.job_label = i18n::opening(name);
        self.status = Status::progress(self.job_label.clone(), 0.0);
        self.detect_open = false;
        self.selected_track = None;
        self.harmony_anchor = None;
        self.compatible_only = false;
        self.filters_dirty = true;
    }

    fn poll_job(&mut self) {
        let mut messages = Vec::new();
        let mut disconnected = false;
        if let Some(receiver) = &self.job {
            loop {
                match receiver.try_recv() {
                    Ok(message) => messages.push(message),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        let mut finished = false;
        for message in messages {
            match message {
                JobMessage::Progress(fraction) => {
                    self.status = Status::progress(self.job_label.clone(), fraction);
                }
                JobMessage::Loaded(library) => {
                    let library = *library;
                    self.search_index = library
                        .tracks
                        .iter()
                        .map(|track| {
                            format!("{} {} {}", track.title, track.artist, track.album)
                                .to_lowercase()
                        })
                        .collect();
                    self.status = Status::info(i18n::loaded(
                        &library.name,
                        library.tracks.len(),
                        library.playlist_count(),
                    ));
                    // El origen mostrado debe coincidir con la librería abierta.
                    self.source = library.source;
                    self.library = Some(library);
                    self.selection = Selection::AllTracks;
                    self.last_selection = Selection::AllTracks;
                    self.selected_track = None;
                    self.harmony_anchor = None;
                    self.compatible_only = false;
                    self.visible.clear();
                    self.filters_dirty = true;
                    self.covers.clear();
                    finished = true;
                }
                JobMessage::Failed(error) => {
                    self.status = Status::warning(i18n::open_failed(&error));
                    finished = true;
                }
            }
        }

        if finished {
            self.job = None;
            self.job_label.clear();
        } else if disconnected {
            self.job = None;
            self.job_label.clear();
            self.status = Status::warning(ES.status_worker_lost);
        }
    }

    fn ensure_visible(&mut self) {
        if self.filters_dirty {
            self.rebuild_visible();
        }
    }

    fn rebuild_visible(&mut self) {
        self.filters_dirty = false;
        let Some(library) = self.library.as_ref() else {
            self.visible.clear();
            return;
        };

        let tokens: Vec<String> = self
            .search
            .trim()
            .to_lowercase()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        let search_index = &self.search_index;
        let anchor = self.harmony_anchor;
        let compatible_only = self.compatible_only;

        let mut visible = Vec::with_capacity(library.selection_len(self.selection));
        let mut consider = |track_id: TrackId| {
            let Some(track) = library.tracks.get(track_id) else {
                return;
            };
            if !tokens.is_empty() {
                let blob = search_index
                    .get(track_id)
                    .map(String::as_str)
                    .unwrap_or_default();
                if !tokens.iter().all(|token| blob.contains(token.as_str())) {
                    return;
                }
            }
            if compatible_only {
                if let Some(anchor) = anchor {
                    let Some(key) = track.key else {
                        return;
                    };
                    if !key.is_compatible(anchor) {
                        return;
                    }
                }
            }
            visible.push(track_id);
        };

        match self.selection {
            Selection::AllTracks => {
                for track_id in 0..library.tracks.len() {
                    consider(track_id);
                }
            }
            Selection::Playlist(id) => {
                if let Some(ids) = library.playlist_track_ids(id) {
                    for &track_id in ids {
                        consider(track_id);
                    }
                }
            }
        }

        self.visible = visible;
    }

    fn apply_action(&mut self, ctx: &Context, action: TopBarAction) {
        match action {
            TopBarAction::DetectLibraries => self.open_detection(ctx),
            TopBarAction::Export => {
                self.status = Status::warning(ES.status_export_todo);
            }
            TopBarAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            TopBarAction::Convert => {
                self.status = Status::warning(i18n::conversion_pending(self.target.label()));
            }
            TopBarAction::About => self.about_open = true,
        }
    }

    fn show_playlists(&mut self, ctx: &Context) {
        egui::SidePanel::left("playlists")
            .resizable(true)
            .default_width(224.0)
            .min_width(180.0)
            .max_width(340.0)
            .frame(theme::frame(8.0))
            .show(ctx, |ui| {
                if let Some(library) = self.library.as_ref() {
                    ui::playlists::show(ui, library, &mut self.selection);
                } else {
                    ui.label(RichText::new(ES.no_library).strong());
                    ui.add_space(4.0);
                    ui.label(RichText::new(ES.no_library_hint).color(theme::TEXT_DIM));
                }
            });

        if self.selection != self.last_selection {
            self.last_selection = self.selection;
            self.filters_dirty = true;
        }
    }

    fn show_detail(&mut self, ctx: &Context) {
        let previous = self.compatible_only;
        egui::SidePanel::right("detalle")
            .resizable(false)
            .exact_width(304.0)
            .frame(theme::frame(10.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui::detail::show(
                            ui,
                            self.library.as_ref(),
                            self.selected_track,
                            &mut self.compatible_only,
                            &mut self.covers,
                        );
                    });
            });

        if previous != self.compatible_only {
            self.filters_dirty = true;
        }
    }

    fn show_center(&mut self, ctx: &Context) {
        let filtering = !self.search.trim().is_empty()
            || (self.compatible_only && self.harmony_anchor.is_some());
        let mut clicked = None;

        egui::CentralPanel::default()
            .frame(theme::frame(10.0))
            .show(ctx, |ui| {
                let Some(library) = self.library.as_ref() else {
                    empty_state(ui, ES.no_library, ES.no_library_hint);
                    return;
                };

                let total = library.selection_len(self.selection);
                let title = match self.selection {
                    Selection::AllTracks => ES.all_tracks.to_string(),
                    Selection::Playlist(id) => library
                        .playlist_name(id)
                        .unwrap_or(ES.unknown_playlist)
                        .to_string(),
                };
                let count = if filtering {
                    i18n::filtered_count(self.visible.len(), total)
                } else {
                    i18n::track_count(total)
                };

                ui.horizontal(|ui| {
                    ui.label(RichText::new(title).heading());
                    ui.label(RichText::new(count).color(theme::TEXT_DIM));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.selectable_value(&mut self.view, ViewMode::Grid, ES.view_grid_short);
                        ui.selectable_value(&mut self.view, ViewMode::List, ES.view_list);
                    });
                });
                ui.separator();

                if self.visible.is_empty() {
                    empty_state(ui, ES.no_results, ES.no_results_hint);
                    return;
                }

                clicked = match self.view {
                    ViewMode::List => ui::list::show(
                        ui,
                        library,
                        &self.visible,
                        self.selected_track,
                        &mut self.covers,
                    ),
                    ViewMode::Grid => ui::grid::show(
                        ui,
                        library,
                        &self.visible,
                        self.selected_track,
                        &mut self.covers,
                    ),
                };
            });

        if let Some(track_id) = clicked {
            self.select(track_id);
        }
    }

    fn show_detection(&mut self, ctx: &Context) {
        if !self.detect_open {
            return;
        }

        let action = {
            let mut state = ui::detect::DetectState {
                candidates: &self.candidates,
                choice: &mut self.detected_choice,
                detecting: self.detecting,
                is_demo: self.provider.is_demo(),
                busy: self.job.is_some(),
                open: &mut self.detect_open,
            };
            ui::detect::show(ctx, &mut state)
        };

        match action {
            Some(DetectAction::Open(id)) => {
                if let Some(candidate) = self.candidates.iter().find(|candidate| candidate.id == id)
                {
                    let name = candidate.name.clone();
                    self.start_open(ctx, id, &name);
                }
            }
            Some(DetectAction::Rescan) => self.start_detection(ctx),
            None => {}
        }
    }
}

fn empty_state(ui: &mut egui::Ui, title: &str, hint: &str) {
    ui.add_space(70.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(title).heading().color(theme::TEXT_DIM));
        ui.add_space(6.0);
        ui.label(RichText::new(hint).color(theme::TEXT_DIM));
    });
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.covers.poll(ctx);
        self.poll_job();
        self.poll_detection();
        self.handle_shortcuts(ctx);

        let mut actions = Vec::new();
        egui::TopBottomPanel::top("barra-superior")
            .frame(theme::frame(8.0))
            .show(ctx, |ui| {
                let mut state = ui::top_bar::TopBarState {
                    source: &mut self.source,
                    target: &mut self.target,
                    search: &mut self.search,
                    view: &mut self.view,
                    library_loaded: self.library.is_some(),
                    focus_search: &mut self.focus_search,
                };
                actions = ui::top_bar::show(ui, &mut state);
            });
        for action in actions {
            self.apply_action(ctx, action);
        }

        if self.search != self.last_search {
            self.last_search.clone_from(&self.search);
            self.filters_dirty = true;
        }

        egui::TopBottomPanel::bottom("estado")
            .frame(theme::frame(6.0))
            .show(ctx, |ui| ui::status::show(ui, &self.status));

        self.show_playlists(ctx);
        self.show_detail(ctx);
        self.ensure_visible();
        self.show_center(ctx);
        self.show_detection(ctx);

        if self.about_open {
            egui::Window::new(ES.about_title)
                .open(&mut self.about_open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(ES.about_body);
                    ui.add_space(8.0);
                    ui.label(RichText::new(ES.about_todo).color(theme::TEXT_DIM));
                });
        }
    }
}
