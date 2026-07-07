use crossterm::event::KeyCode;

use crate::app::{App, Mode};
use crate::input::TerminalKey;

pub(crate) fn handle_audio_summary_key(app: &mut App, key: TerminalKey) -> bool {
    if key.kind != crossterm::event::KeyEventKind::Press {
        return false;
    }

    let is_audio_summary_trigger = app.state.keybinds.audio_summary.matches_direct_key(key)
        || (app.state.mode == Mode::Prefix
            && app.state.keybinds.audio_summary.matches_prefix_key(key));

    if is_audio_summary_trigger {
        let is_agent = app
            .state
            .active
            .and_then(|ws_idx| {
                let ws = app.state.workspaces.get(ws_idx)?;
                let pane_id = ws.focused_pane_id()?;
                let pane = ws.pane_state(pane_id)?;
                let term = app.state.terminals.get(&pane.attached_terminal_id)?;
                Some(term.is_agent_terminal())
            })
            .unwrap_or(false);

        if is_agent {
            if let Some(ws_idx) = app.state.active {
                let tab_idx = app.state.workspaces[ws_idx].active_tab;
                app.trigger_audio_summary(ws_idx, tab_idx);
                if app.state.mode == Mode::Prefix {
                    crate::app::input::leave_command_mode(&mut app.state);
                }
                return true;
            }
        }
    }

    app.cancel_audio_summary();
    false
}

pub(crate) fn handle_speech_to_text_key(app: &mut App, key: TerminalKey) -> bool {
    if key.kind == crossterm::event::KeyEventKind::Release {
        app.release_events_supported = true;
    }

    if app.no_session
        && app
            .state
            .extensions
            .speech_to_text
            .gemini_api_key
            .as_ref()
            .is_none_or(|k| k.trim().is_empty())
    {
        return false;
    }

    if app.state.extensions.recording_workspace.is_some() {
        let is_stt_key = app.state.keybinds.speech_to_text.matches_direct_key(key)
            || app.state.keybinds.speech_to_text.matches_prefix_key(key)
            || (app.extensions.speech_recorder.recording_key().is_some()
                && key.code == app.extensions.speech_recorder.recording_key().unwrap().code);

        if is_stt_key || key.code == KeyCode::Esc {
            if key.kind == crossterm::event::KeyEventKind::Repeat {
                return true;
            }

            if key.code == KeyCode::Esc {
                let previous_toast = app.state.toast.clone();
                app.stop_recording(true);
                app.state.toast = Some(crate::app::state::ToastNotification {
                    kind: crate::app::state::ToastKind::NeedsAttention,
                    title: "Speech to Text".into(),
                    context: "Recording aborted.".into(),
                    position: None,
                    target: None,
                });
                app.sync_toast_deadline(previous_toast);
                return true;
            }

            let elapsed = app
                .extensions
                .speech_recorder
                .start_time()
                .map(|t| t.elapsed())
                .unwrap_or(std::time::Duration::ZERO);

            let should_stop = match key.kind {
                crossterm::event::KeyEventKind::Release => {
                    if elapsed < std::time::Duration::from_millis(400) {
                        app.extensions.speech_recorder.is_toggle = true;
                        false
                    } else {
                        true
                    }
                }
                crossterm::event::KeyEventKind::Press => {
                    if app.extensions.speech_recorder.is_toggle {
                        true
                    } else if !app.release_events_supported {
                        elapsed >= std::time::Duration::from_millis(400)
                    } else {
                        false
                    }
                }
                _ => false,
            };

            if should_stop {
                app.stop_recording(false);
                return true;
            }
        }

        return true;
    }

    if key.kind == crossterm::event::KeyEventKind::Press {
        let is_direct_match = app.state.keybinds.speech_to_text.matches_direct_key(key);
        let is_prefix_match = app.state.mode == Mode::Prefix
            && app.state.keybinds.speech_to_text.matches_prefix_key(key);

        if is_direct_match || is_prefix_match {
            if let Some(ws_idx) = app.state.active {
                if app.start_recording(ws_idx, key) && app.state.mode == Mode::Prefix {
                    crate::app::input::leave_command_mode(&mut app.state);
                }
            }
            return true;
        }
    }

    false
}

pub(crate) fn handle_outer_focus_lost(app: &mut App) {
    if app.state.extensions.recording_workspace.is_none() {
        return;
    }

    app.stop_recording(true);
    app.state.toast = Some(crate::app::state::ToastNotification {
        kind: crate::app::state::ToastKind::NeedsAttention,
        title: "Speech to Text".into(),
        context: "Recording aborted due to focus loss.".into(),
        position: None,
        target: None,
    });
}
