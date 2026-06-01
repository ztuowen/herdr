use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use serde::Serialize;
use serde_json::{json, Value};

const HERDR_SOCKET_PATH_ENV: &str = "HERDR_SOCKET_PATH";
const HERDR_SESSION_ENV: &str = "HERDR_SESSION";
const DEFAULT_SESSION_NAME: &str = "default";

#[derive(Debug, Clone)]
pub struct ApiBridge {
    socket_path: PathBuf,
}

impl Default for ApiBridge {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
        }
    }
}

impl ApiBridge {
    #[cfg(test)]
    fn for_socket_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub fn server_status(&self) -> BridgeResponse {
        self.call("server_status", "ping", json!({}))
    }

    pub fn workspace_list(&self) -> BridgeResponse {
        self.call("workspace_list", "workspace.list", json!({}))
    }

    pub fn workspace_focus(&self, workspace_id: String) -> BridgeResponse {
        self.call(
            "workspace_focus",
            "workspace.focus",
            json!({ "workspace_id": workspace_id }),
        )
    }

    pub fn workspace_create(
        &self,
        cwd: Option<String>,
        label: Option<String>,
        focus: bool,
    ) -> BridgeResponse {
        let mut params = json!({ "focus": focus });
        insert_non_empty(&mut params, "cwd", cwd);
        insert_non_empty(&mut params, "label", label);
        self.call("workspace_create", "workspace.create", params)
    }

    pub fn workspace_rename(&self, workspace_id: String, label: String) -> BridgeResponse {
        self.call(
            "workspace_rename",
            "workspace.rename",
            json!({
                "workspace_id": workspace_id,
                "label": label,
            }),
        )
    }

    pub fn tab_list(&self, workspace_id: Option<String>) -> BridgeResponse {
        let params = optional_workspace_params(workspace_id);
        self.call("tab_list", "tab.list", params)
    }

    pub fn tab_focus(&self, tab_id: String) -> BridgeResponse {
        self.call("tab_focus", "tab.focus", json!({ "tab_id": tab_id }))
    }

    pub fn pane_list(&self, workspace_id: Option<String>) -> BridgeResponse {
        let params = optional_workspace_params(workspace_id);
        self.call("pane_list", "pane.list", params)
    }

    pub fn agent_list(&self) -> BridgeResponse {
        self.call("agent_list", "agent.list", json!({}))
    }

    pub fn agent_focus(&self, target: String) -> BridgeResponse {
        self.call("agent_focus", "agent.focus", json!({ "target": target }))
    }

    pub fn agent_send(&self, target: String, text: String) -> BridgeResponse {
        self.call(
            "agent_send",
            "agent.send",
            json!({
                "target": target,
                "text": text,
            }),
        )
    }

    pub fn kanban_list(&self, status: Option<String>) -> BridgeResponse {
        let params = match status {
            Some(status) if !status.trim().is_empty() => json!({ "status": status }),
            _ => json!({}),
        };
        self.call("kanban_list", "kanban.list", params)
    }

    pub fn kanban_add(
        &self,
        title: String,
        description: Option<String>,
        status: Option<String>,
    ) -> BridgeResponse {
        let mut params = json!({ "title": title });
        insert_non_empty(&mut params, "description", description);
        insert_non_empty(&mut params, "status", status);
        self.call("kanban_add", "kanban.add", params)
    }

    pub fn kanban_update(
        &self,
        uuid: String,
        title: Option<String>,
        description: Option<String>,
        status: Option<String>,
    ) -> BridgeResponse {
        let mut params = json!({ "uuid": uuid });
        insert_non_empty(&mut params, "title", title);
        insert_non_empty(&mut params, "description", description);
        insert_non_empty(&mut params, "status", status);
        self.call("kanban_update", "kanban.update", params)
    }

    pub fn kanban_delete(&self, uuid: String) -> BridgeResponse {
        self.call("kanban_delete", "kanban.delete", json!({ "uuid": uuid }))
    }

    pub fn pane_read(&self, pane_id: String) -> BridgeResponse {
        self.call(
            "pane_read",
            "pane.read",
            json!({
                "pane_id": pane_id,
                "source": "recent",
                "lines": 80,
                "format": "text",
                "strip_ansi": true
            }),
        )
    }

    fn call(&self, id: &str, method: &str, params: Value) -> BridgeResponse {
        let request = json!({
            "id": format!("orbit:{id}"),
            "method": method,
            "params": params
        });

        match request_json(&self.socket_path, &request) {
            Ok(data) => BridgeResponse::success(self.socket_path.clone(), data),
            Err(error) => BridgeResponse::failure(self.socket_path.clone(), error),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeResponse {
    pub ok: bool,
    pub socket_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BridgeError>,
}

impl BridgeResponse {
    fn success(socket_path: PathBuf, data: Value) -> Self {
        Self {
            ok: true,
            socket_path: socket_path.display().to_string(),
            data: Some(data),
            error: None,
        }
    }

    fn failure(socket_path: PathBuf, error: BridgeError) -> Self {
        Self {
            ok: false,
            socket_path: socket_path.display().to_string(),
            data: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BridgeError {
    pub kind: &'static str,
    pub message: String,
}

impl From<io::Error> for BridgeError {
    fn from(error: io::Error) -> Self {
        let kind = match error.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => "server_not_running",
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => "timeout",
            _ => "io_error",
        };

        Self {
            kind,
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for BridgeError {
    fn from(error: serde_json::Error) -> Self {
        Self {
            kind: "invalid_json",
            message: error.to_string(),
        }
    }
}

fn request_json(socket_path: &PathBuf, request: &Value) -> Result<Value, BridgeError> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(serde_json::to_string(request)?.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut line = String::new();
    let read = BufReader::new(stream).read_line(&mut line)?;
    if read == 0 || line.trim().is_empty() {
        return Err(BridgeError {
            kind: "empty_response",
            message: "server returned an empty API response".into(),
        });
    }

    serde_json::from_str(&line).map_err(BridgeError::from)
}

fn optional_workspace_params(workspace_id: Option<String>) -> Value {
    match workspace_id {
        Some(workspace_id) if !workspace_id.trim().is_empty() => {
            json!({ "workspace_id": workspace_id })
        }
        _ => json!({}),
    }
}

fn insert_non_empty(params: &mut Value, key: &str, value: Option<String>) {
    let Some(value) = value else {
        return;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if let Some(object) = params.as_object_mut() {
        object.insert(key.to_string(), Value::String(trimmed.to_string()));
    }
}

fn default_socket_path() -> PathBuf {
    if let Ok(path) = std::env::var(HERDR_SOCKET_PATH_ENV) {
        return PathBuf::from(path);
    }

    let config_dir = config_dir();
    match std::env::var(HERDR_SESSION_ENV) {
        Ok(name) if !name.trim().is_empty() && name != DEFAULT_SESSION_NAME => {
            config_dir.join("sessions").join(name).join("herdr.sock")
        }
        _ => config_dir.join("herdr.sock"),
    }
}

fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(dir).join(app_dir_name());
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(format!(".config/{}", app_dir_name()));
    }
    PathBuf::from(format!("/tmp/{}", app_dir_name()))
}

fn app_dir_name() -> &'static str {
    if cfg!(debug_assertions) {
        "herdr-dev"
    } else {
        "herdr"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_socket_is_classified_as_server_not_running() {
        let bridge =
            ApiBridge::for_socket_path(PathBuf::from("/tmp/herdr-orbit-missing-server-test.sock"));

        let response = bridge.server_status();

        assert!(!response.ok);
        assert_eq!(
            response.error.map(|error| error.kind),
            Some("server_not_running")
        );
    }

    #[test]
    fn workspace_params_are_omitted_without_workspace_id() {
        assert_eq!(optional_workspace_params(None), json!({}));
        assert_eq!(optional_workspace_params(Some(" ".into())), json!({}));
    }

    #[test]
    fn workspace_params_include_workspace_id_when_present() {
        assert_eq!(
            optional_workspace_params(Some("workspace-1".into())),
            json!({ "workspace_id": "workspace-1" })
        );
    }

    #[test]
    fn insert_non_empty_omits_blank_optional_values() {
        let mut params = json!({});
        insert_non_empty(&mut params, "label", None);
        insert_non_empty(&mut params, "cwd", Some(" ".into()));

        assert_eq!(params, json!({}));
    }

    #[test]
    fn insert_non_empty_trims_values() {
        let mut params = json!({});
        insert_non_empty(&mut params, "label", Some(" Orbit ".into()));

        assert_eq!(params, json!({ "label": "Orbit" }));
    }

    #[test]
    #[ignore = "requires an already running default Herdr server"]
    fn live_default_server_bridge_commands_return_diagnostics() {
        let bridge = ApiBridge::default();

        let status = bridge.server_status();
        assert!(status.ok, "{status:?}");
        println!("server_status: {}", summarize_result(&status, "type"));

        let workspaces = bridge.workspace_list();
        assert!(workspaces.ok, "{workspaces:?}");
        println!(
            "workspace_list: {} workspaces",
            result_array_len(&workspaces, "workspaces")
        );

        let tabs = bridge.tab_list(None);
        assert!(tabs.ok, "{tabs:?}");
        println!("tab_list: {} tabs", result_array_len(&tabs, "tabs"));

        let panes = bridge.pane_list(None);
        assert!(panes.ok, "{panes:?}");
        println!("pane_list: {} panes", result_array_len(&panes, "panes"));

        let agents = bridge.agent_list();
        assert!(agents.ok, "{agents:?}");
        println!("agent_list: {} agents", result_array_len(&agents, "agents"));

        let kanban = bridge.kanban_list(None);
        assert!(kanban.ok, "{kanban:?}");
        println!("kanban_list: {} items", result_array_len(&kanban, "items"));

        let pane_id =
            first_pane_id(&panes).expect("running server should expose at least one pane");
        let read = bridge.pane_read(pane_id);
        assert!(read.ok, "{read:?}");
        println!("pane_read: {}", summarize_result(&read, "type"));
    }

    #[test]
    #[ignore = "mutates an already running default Herdr server"]
    fn live_default_server_bridge_write_commands_round_trip() {
        let bridge = ApiBridge::default();
        let original_workspace = first_workspace_id(&bridge.workspace_list());
        let label = format!("Orbit validation {}", std::process::id());

        let created = bridge.workspace_create(None, Some(label.clone()), true);
        assert!(created.ok, "{created:?}");
        let workspace_id = nested_string(&created, &["result", "workspace", "workspace_id"])
            .expect("workspace.create should return a workspace id");
        let tab_id = nested_string(&created, &["result", "tab", "tab_id"])
            .expect("workspace.create should return a tab id");
        let pane_id = nested_string(&created, &["result", "root_pane", "pane_id"])
            .expect("workspace.create should return a root pane id");

        let renamed = bridge.workspace_rename(workspace_id.clone(), format!("{label} renamed"));
        assert!(renamed.ok, "{renamed:?}");

        let focused_workspace = bridge.workspace_focus(workspace_id.clone());
        assert!(focused_workspace.ok, "{focused_workspace:?}");

        let focused_tab = bridge.tab_focus(tab_id);
        assert!(focused_tab.ok, "{focused_tab:?}");

        let focused_agent = bridge.agent_focus(pane_id.clone());
        assert!(focused_agent.ok, "{focused_agent:?}");

        let sent = bridge.agent_send(pane_id, "printf 'orbit validation write path'\n".into());
        assert!(sent.ok, "{sent:?}");

        let added = bridge.kanban_add(format!("{label} kanban"), None, Some("todo".into()));
        assert!(added.ok, "{added:?}");
        let uuid = nested_string(&added, &["result", "item", "uuid"])
            .expect("kanban.add should return an item uuid");

        let updated = bridge.kanban_update(
            uuid.clone(),
            Some(format!("{label} kanban updated")),
            None,
            Some("reviewing".into()),
        );
        assert!(updated.ok, "{updated:?}");

        let deleted = bridge.kanban_delete(uuid);
        assert!(deleted.ok, "{deleted:?}");

        if let Some(original_workspace) = original_workspace {
            let refocused = bridge.workspace_focus(original_workspace);
            assert!(refocused.ok, "{refocused:?}");
        }

        let closed = bridge.call(
            "workspace_close",
            "workspace.close",
            json!({ "workspace_id": workspace_id }),
        );
        assert!(closed.ok, "{closed:?}");
    }

    fn first_pane_id(response: &BridgeResponse) -> Option<String> {
        response
            .data
            .as_ref()?
            .get("result")?
            .get("panes")?
            .as_array()?
            .first()?
            .get("pane_id")?
            .as_str()
            .map(ToOwned::to_owned)
    }

    fn first_workspace_id(response: &BridgeResponse) -> Option<String> {
        response
            .data
            .as_ref()?
            .get("result")?
            .get("workspaces")?
            .as_array()?
            .iter()
            .find_map(|workspace| {
                workspace
                    .get("workspace_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
    }

    fn nested_string(response: &BridgeResponse, path: &[&str]) -> Option<String> {
        let mut current = response.data.as_ref()?;
        for segment in path {
            current = current.get(segment)?;
        }
        current.as_str().map(ToOwned::to_owned)
    }

    fn result_array_len(response: &BridgeResponse, field: &str) -> usize {
        response
            .data
            .as_ref()
            .and_then(|data| data.get("result"))
            .and_then(|result| result.get(field))
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default()
    }

    fn summarize_result(response: &BridgeResponse, field: &str) -> String {
        response
            .data
            .as_ref()
            .and_then(|data| data.get("result"))
            .and_then(|result| result.get(field))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string()
    }
}
