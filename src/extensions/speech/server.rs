use crate::events::AppEvent;
use crate::protocol::ServerMessage;
use crate::server::client_transport::ServerEvent;

pub(crate) fn server_message_for_app_event(ev: &AppEvent) -> Option<ServerMessage> {
    match ev {
        AppEvent::SpeechStartRecording {
            workspace_id,
            pane_id,
            is_agent,
        } => Some(ServerMessage::StartRecording {
            workspace_id: workspace_id.clone(),
            pane_id: *pane_id,
            is_agent: *is_agent,
        }),
        AppEvent::SpeechStopRecording { abort } => {
            Some(ServerMessage::StopRecording { abort: *abort })
        }
        AppEvent::AudioSummaryStart { text_content } => Some(ServerMessage::StartAudioSummary {
            text_content: text_content.clone(),
        }),
        AppEvent::AudioSummaryCancel => Some(ServerMessage::CancelAudioSummary),
        _ => None,
    }
}

pub(crate) fn app_event_for_client_event(
    ev: &ServerEvent,
    foreground_client_id: Option<u64>,
) -> Option<AppEvent> {
    match ev {
        ServerEvent::ClientSpeechPartialTranscription {
            client_id,
            workspace_id,
            text,
        } if Some(*client_id) == foreground_client_id => {
            Some(AppEvent::SpeechPartialTranscription {
                workspace_id: workspace_id.clone(),
                text: text.clone(),
            })
        }
        ServerEvent::ClientSpeechTranscribed {
            client_id,
            workspace_id,
            pane_id,
            result,
        } if Some(*client_id) == foreground_client_id => Some(AppEvent::SpeechTranscribed {
            workspace_id: workspace_id.clone(),
            pane_id: *pane_id,
            result: result.clone(),
        }),
        ServerEvent::ClientAudioSummaryFinished { client_id }
            if Some(*client_id) == foreground_client_id =>
        {
            Some(AppEvent::AudioSummaryFinished)
        }
        ServerEvent::ClientAudioSummaryError { client_id, error }
            if Some(*client_id) == foreground_client_id =>
        {
            Some(AppEvent::AudioSummaryError(error.clone()))
        }
        _ => None,
    }
}
