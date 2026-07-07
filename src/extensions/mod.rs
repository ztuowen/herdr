use crate::api::schema::Method;
use crate::events::AppEvent;
use crate::input::TerminalKey;
use ratatui::layout::Rect;
use ratatui::Frame;

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

pub fn handle_extension_api_request(
    app: &mut crate::app::App,
    request_id: String,
    method: &Method,
) -> Option<String> {
    match method {
        Method::KanbanAdd(params) => Some(app.handle_kanban_add(request_id, params.clone())),
        Method::KanbanList(params) => Some(app.handle_kanban_list(request_id, params.clone())),
        Method::KanbanUpdate(params) => Some(app.handle_kanban_update(request_id, params.clone())),
        Method::KanbanDelete(params) => Some(app.handle_kanban_delete(request_id, params.clone())),
        _ => None,
    }
}

pub fn handle_extension_key(app: &mut crate::app::App, key: TerminalKey) -> bool {
    if app.state.mode == crate::app::Mode::Kanban {
        crate::extensions::kanban::input::handle_kanban_key(&mut app.state, key);
        return true;
    }

    if app.handle_speech_to_text_key(key) {
        return true;
    }

    false
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
