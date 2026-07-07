use crate::api::schema::{AppSnapshot, AppSnapshotServerInfo, ResponseResult};
use crate::app::App;

use super::responses::{encode_error, encode_success};

impl App {
    pub(super) fn handle_app_snapshot(&mut self, id: String) -> String {
        let workspaces = self
            .state
            .workspaces
            .iter()
            .enumerate()
            .map(|(idx, _)| self.workspace_info(idx))
            .collect();

        let mut tabs = Vec::new();
        for (ws_idx, ws) in self.state.workspaces.iter().enumerate() {
            for tab_idx in 0..ws.tabs.len() {
                if let Some(tab) = self.tab_info(ws_idx, tab_idx) {
                    tabs.push(tab);
                }
            }
        }

        let panes = match self.collect_panes_for_workspace(None) {
            Ok(panes) => panes,
            Err((code, message)) => return encode_error(id, &code, message),
        };

        let snapshot = AppSnapshot {
            server: AppSnapshotServerInfo {
                version: env!("CARGO_PKG_VERSION").into(),
                protocol: crate::protocol::PROTOCOL_VERSION,
                capabilities: None,
            },
            workspaces,
            tabs,
            panes,
            agents: self.collect_agent_infos(),
            kanban_items: self.state.extensions.kanban_items_for_persistence(),
        };

        encode_success(id, ResponseResult::AppSnapshot { snapshot })
    }
}

#[cfg(test)]
mod tests {
    use crate::api::schema::{
        EmptyParams, KanbanStatus, Method, Request, ResponseResult, SuccessResponse,
    };
    use crate::app::App;
    use crate::config::Config;
    use crate::workspace::Workspace;

    fn test_app() -> App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        App::new(
            &Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        )
    }

    #[test]
    fn app_snapshot_returns_joined_top_level_collections() {
        let mut app = test_app();
        app.state.workspaces = vec![Workspace::test_new("main"), Workspace::test_new("review")];
        app.state.ensure_test_terminals();
        for terminal in app.state.terminals.values_mut() {
            terminal.set_agent_name("codex".into());
        }
        app.state.switch_workspace(0);
        let first_terminal_id = app.state.workspaces[0].tabs[0]
            .panes
            .values()
            .next()
            .unwrap()
            .attached_terminal_id
            .to_string();
        app.state.extensions.kanban.add_item(
            "Review snapshot".into(),
            Some("snapshot.md".into()),
            Some(KanbanStatus::Reviewing),
            Some(first_terminal_id),
        );

        let response = app.handle_api_request(Request {
            id: "req_snapshot".into(),
            method: Method::AppSnapshot(EmptyParams::default()),
        });

        let success: SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(success.id, "req_snapshot");
        let ResponseResult::AppSnapshot { snapshot } = success.result else {
            panic!("expected app_snapshot response");
        };
        assert_eq!(snapshot.server.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(snapshot.server.protocol, crate::protocol::PROTOCOL_VERSION);
        assert_eq!(snapshot.workspaces.len(), 2);
        assert_eq!(snapshot.tabs.len(), 2);
        assert_eq!(snapshot.panes.len(), 2);
        assert_eq!(snapshot.agents.len(), 2);
        assert_eq!(snapshot.kanban_items.len(), 1);
        assert_eq!(
            snapshot.tabs[0].workspace_id,
            snapshot.workspaces[0].workspace_id
        );
        assert_eq!(snapshot.panes[0].tab_id, snapshot.tabs[0].tab_id);
        assert_eq!(snapshot.agents[0].pane_id, snapshot.panes[0].pane_id);
    }
}
