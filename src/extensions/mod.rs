use crate::api::schema::{EventData, Method};
use crate::events::AppEvent;
use crate::input::TerminalKey;
use crossterm::event::MouseEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) mod kanban;
pub(crate) mod markdown;
pub(crate) mod speech;

#[derive(Debug, Clone)]
pub struct PendingEnter {
    pub workspace_id: String,
    pub pane_id: crate::layout::PaneId,
    pub sequence: u64,
}

pub struct ExtensionsState {
    pub static_image_placements: std::sync::Mutex<Vec<crate::app::state::StaticImagePlacement>>,

    pub speech_to_text: crate::config::SpeechToTextConfig,
    pub recording_workspace: Option<String>,
    pub live_transcription: Option<String>,
    pub kanban: crate::extensions::kanban::KanbanState,
    pub pending_enter: Option<PendingEnter>,
    pub pending_enter_sequence: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct KanbanColumnCounts {
    pub todo: usize,
    pub ongoing: usize,
    pub blocked: usize,
    pub reviewing: usize,
    pub done: usize,
}

impl ExtensionsState {
    pub fn new(
        speech_to_text: crate::config::SpeechToTextConfig,
        kanban_items: Vec<crate::api::schema::KanbanItem>,
    ) -> Self {
        Self {
            static_image_placements: std::sync::Mutex::new(Vec::new()),
            speech_to_text,
            recording_workspace: None,
            live_transcription: None,
            kanban: crate::extensions::kanban::KanbanState::new(kanban_items),
            pending_enter: None,
            pending_enter_sequence: 0,
        }
    }

    pub(crate) fn has_persisted_extension_data(&self) -> bool {
        !self.kanban.items.is_empty()
    }

    pub(crate) fn kanban_items_for_persistence(&self) -> Vec<crate::api::schema::KanbanItem> {
        self.kanban.items.clone()
    }

    pub(crate) fn restore_kanban_items(&mut self, items: Vec<crate::api::schema::KanbanItem>) {
        self.kanban = crate::extensions::kanban::KanbanState::new(items);
    }

    pub(crate) fn apply_speech_to_text_config(
        &mut self,
        config: crate::config::SpeechToTextConfig,
    ) {
        self.speech_to_text = config;
    }

    pub(crate) fn kanban_column_counts(&self) -> KanbanColumnCounts {
        KanbanColumnCounts {
            todo: self.kanban.items_in_column(0).len(),
            ongoing: self.kanban.items_in_column(1).len(),
            blocked: self.kanban.items_in_column(2).len(),
            reviewing: self.kanban.items_in_column(3).len(),
            done: self.kanban.items_in_column(4).len(),
        }
    }
}

pub struct ExtensionsRuntime {
    pub speech_recorder: crate::extensions::speech::SpeechRecorder,
    pub tab_summarizer: Option<crate::extensions::speech::summary::TabSummarizer>,
}

impl ExtensionsRuntime {
    pub fn new() -> Self {
        Self {
            speech_recorder: crate::extensions::speech::SpeechRecorder::new(),
            tab_summarizer: None,
        }
    }
}

pub fn handle_extension_event(app: &mut crate::app::App, ev: &AppEvent) -> bool {
    if crate::extensions::speech::events::handle_speech_event(app, ev) {
        return true;
    }

    false
}

pub(crate) fn init_extension_runtime_hooks(
    render_notify: Arc<tokio::sync::Notify>,
    render_dirty: Arc<AtomicBool>,
) {
    crate::extensions::markdown::math::init_redraw_notifier(render_notify, render_dirty);
}

pub(crate) fn handle_plugin_availability_changed(app: &mut crate::app::App) {
    crate::extensions::kanban::api::mirror_existing_cards_to_plugin_resources(app);
}

pub fn handle_extension_api_request(
    app: &mut crate::app::App,
    request_id: String,
    method: &Method,
) -> Option<String> {
    crate::extensions::kanban::api::handle_api_request(app, request_id, method)
}

pub fn handle_extension_key(app: &mut crate::app::App, key: TerminalKey) -> bool {
    if app.state.mode == crate::app::Mode::Kanban {
        crate::extensions::kanban::input::handle_kanban_key(&mut app.state, key);
        return true;
    }

    if crate::extensions::speech::input::handle_speech_to_text_key(app, key) {
        return true;
    }

    false
}

pub(crate) fn handle_extension_mouse(state: &mut crate::app::AppState, mouse: MouseEvent) -> bool {
    crate::extensions::kanban::mouse::handle_kanban_mouse(state, mouse)
}

pub fn render_extension_ui(
    app: &crate::app::AppState,
    frame: &mut Frame,
    terminal_area: Rect,
) -> bool {
    if app.mode == crate::app::Mode::Kanban {
        crate::extensions::kanban::ui::render_kanban(app, frame, terminal_area);
        return true;
    }
    false
}

pub(crate) fn active_extension_hyperlinks(
    app: &crate::app::AppState,
) -> Vec<((u16, u16), String, String)> {
    if app.mode == crate::app::Mode::Kanban {
        return crate::extensions::kanban::ui::active_kanban_detail_hyperlinks(app);
    }

    Vec::new()
}

pub(crate) fn plugin_invocation_source_for_event(event_data: &EventData) -> Option<String> {
    crate::extensions::kanban::events::plugin_invocation_source(event_data)
}
