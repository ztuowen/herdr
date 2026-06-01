import React, { useCallback, useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type BridgeError = {
  kind: string;
  message: string;
};

type BridgeResponse = {
  ok: boolean;
  socket_path: string;
  data?: unknown;
  error?: BridgeError;
};

type Endpoint = {
  key: string;
  label: string;
  command: string;
  params?: Record<string, unknown>;
};

type WorkspaceInfo = {
  workspace_id: string;
  label: string;
  focused: boolean;
};

type TabInfo = {
  tab_id: string;
  workspace_id: string;
  label: string;
  focused: boolean;
};

type AgentInfo = {
  terminal_id: string;
  pane_id: string;
  workspace_id: string;
  tab_id: string;
  focused: boolean;
  name?: string;
  agent?: string;
  title?: string;
  display_agent?: string;
  agent_status: string;
};

type KanbanItem = {
  uuid: string;
  title: string;
  description: string;
  status: KanbanStatus;
  terminal_id?: string;
};

type KanbanStatus = "todo" | "ongoing" | "blocked" | "reviewing" | "done";

type ResultMap = Record<string, BridgeResponse | undefined>;

const BASE_ENDPOINTS: Endpoint[] = [
  { key: "server", label: "Server", command: "server_status" },
  { key: "workspaces", label: "Workspaces", command: "workspace_list" },
  { key: "tabs", label: "Tabs", command: "tab_list" },
  { key: "panes", label: "Panes", command: "pane_list" },
  { key: "agents", label: "Agents", command: "agent_list" },
  { key: "kanban", label: "Kanban", command: "kanban_list" }
];

const KANBAN_STATUSES: KanbanStatus[] = ["todo", "ongoing", "blocked", "reviewing", "done"];

function App() {
  const [results, setResults] = useState<ResultMap>({});
  const [loading, setLoading] = useState(false);
  const [mutationError, setMutationError] = useState<string | undefined>();
  const [mutationOk, setMutationOk] = useState<string | undefined>();
  const [selectedPaneId, setSelectedPaneId] = useState<string | undefined>();
  const [workspaceCreate, setWorkspaceCreate] = useState({ label: "", cwd: "", focus: true });
  const [workspaceRename, setWorkspaceRename] = useState({ workspaceId: "", label: "" });
  const [agentSend, setAgentSend] = useState({ target: "", text: "" });
  const [kanbanAdd, setKanbanAdd] = useState({
    title: "",
    description: "",
    status: "todo" as KanbanStatus
  });
  const [kanbanUpdate, setKanbanUpdate] = useState({
    uuid: "",
    title: "",
    description: "",
    status: "" as "" | KanbanStatus
  });
  const [deleteConfirmUuid, setDeleteConfirmUuid] = useState("");

  const endpoints = useMemo<Endpoint[]>(() => {
    const items = [...BASE_ENDPOINTS];
    if (selectedPaneId) {
      items.push({
        key: "pane_read",
        label: "Pane Read",
        command: "pane_read",
        params: { paneId: selectedPaneId }
      });
    }
    return items;
  }, [selectedPaneId]);

  const refresh = useCallback(async () => {
    setLoading(true);
    const next: ResultMap = {};

    for (const endpoint of BASE_ENDPOINTS) {
      next[endpoint.key] = await invoke<BridgeResponse>(endpoint.command, endpoint.params ?? {});
    }

    const paneId = firstPaneId(next.panes);
    setSelectedPaneId(paneId);

    if (paneId) {
      next.pane_read = await invoke<BridgeResponse>("pane_read", { paneId });
    }

    setResults(next);
    setLoading(false);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const workspaces = resultArray<WorkspaceInfo>(results.workspaces, "workspaces");
  const tabs = resultArray<TabInfo>(results.tabs, "tabs");
  const agents = resultArray<AgentInfo>(results.agents, "agents");
  const kanbanItems = resultArray<KanbanItem>(results.kanban, "items");
  const selectedKanbanItem = kanbanItems.find((item) => item.uuid === kanbanUpdate.uuid);
  const status = results.server;
  const connected = Boolean(status?.ok);
  const counts = {
    workspaces: workspaces.length,
    tabs: tabs.length,
    panes: countResultArray(results.panes, "panes"),
    agents: agents.length,
    kanban: kanbanItems.length
  };

  useEffect(() => {
    if (!workspaceRename.workspaceId && workspaces[0]) {
      setWorkspaceRename({ workspaceId: workspaces[0].workspace_id, label: workspaces[0].label });
    }
  }, [workspaceRename.workspaceId, workspaces]);

  useEffect(() => {
    if (!agentSend.target && agents[0]) {
      setAgentSend((current) => ({ ...current, target: agents[0].terminal_id }));
    }
  }, [agentSend.target, agents]);

  useEffect(() => {
    if (!kanbanUpdate.uuid && kanbanItems[0]) {
      setKanbanUpdate({
        uuid: kanbanItems[0].uuid,
        title: kanbanItems[0].title,
        description: kanbanItems[0].description,
        status: kanbanItems[0].status
      });
    }
  }, [kanbanItems, kanbanUpdate.uuid]);

  useEffect(() => {
    if (selectedKanbanItem) {
      setKanbanUpdate((current) => ({
        ...current,
        title: current.title || selectedKanbanItem.title,
        description: current.description || selectedKanbanItem.description,
        status: current.status || selectedKanbanItem.status
      }));
    }
  }, [selectedKanbanItem?.uuid]);

  const runMutation = useCallback(
    async (label: string, command: string, params: Record<string, unknown>) => {
      setMutationError(undefined);
      setMutationOk(undefined);
      const response = await invoke<BridgeResponse>(command, params);
      if (!response.ok) {
        setMutationError(`${label}: ${response.error?.message ?? response.error?.kind ?? "failed"}`);
        return;
      }
      setMutationOk(`${label} saved`);
      await refresh();
    },
    [refresh]
  );

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="Orbit navigation">
        <div className="brand">Orbit</div>
        <nav>
          <button className="nav-button active" type="button">
            Dashboard
          </button>
        </nav>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <h1>Live Herdr Control</h1>
            <p>{status?.socket_path ?? "default Herdr socket"}</p>
          </div>
          <div className="actions">
            <span className={connected ? "state running" : "state stopped"}>
              {statusLabel(status)}
            </span>
            <button type="button" onClick={() => void refresh()} disabled={loading}>
              {loading ? "Refreshing" : "Refresh"}
            </button>
          </div>
        </header>

        {(mutationError || mutationOk) && (
          <div className={mutationError ? "notice error" : "notice ok"} role="status">
            {mutationError ?? mutationOk}
          </div>
        )}

        <section className="summary" aria-label="Live response counts">
          <Metric label="Workspaces" value={counts.workspaces} />
          <Metric label="Tabs" value={counts.tabs} />
          <Metric label="Panes" value={counts.panes} />
          <Metric label="Agents" value={counts.agents} />
          <Metric label="Kanban" value={counts.kanban} />
        </section>

        <section className="action-panel" aria-label="Write actions">
          <ActionSection title="Workspace">
            <div className="row">
              <Select
                label="Workspace"
                value={workspaceRename.workspaceId}
                onChange={(value) => {
                  const workspace = workspaces.find((item) => item.workspace_id === value);
                  setWorkspaceRename({ workspaceId: value, label: workspace?.label ?? "" });
                }}
                options={workspaces.map((workspace) => ({
                  value: workspace.workspace_id,
                  label: `${workspace.label}${workspace.focused ? " (focused)" : ""}`
                }))}
              />
              <button
                type="button"
                disabled={!workspaceRename.workspaceId}
                onClick={() =>
                  void runMutation("Workspace focus", "workspace_focus", {
                    workspaceId: workspaceRename.workspaceId
                  })
                }
              >
                Focus
              </button>
            </div>
            <form
              className="row"
              onSubmit={(event) => {
                event.preventDefault();
                void runMutation("Workspace rename", "workspace_rename", {
                  workspaceId: workspaceRename.workspaceId,
                  label: workspaceRename.label
                });
              }}
            >
              <input
                aria-label="Workspace label"
                value={workspaceRename.label}
                onChange={(event) =>
                  setWorkspaceRename((current) => ({ ...current, label: event.target.value }))
                }
                placeholder="Workspace label"
              />
              <button type="submit" disabled={!workspaceRename.workspaceId || !workspaceRename.label.trim()}>
                Rename
              </button>
            </form>
            <form
              className="stack"
              onSubmit={(event) => {
                event.preventDefault();
                void runMutation("Workspace create", "workspace_create", {
                  label: emptyToUndefined(workspaceCreate.label),
                  cwd: emptyToUndefined(workspaceCreate.cwd),
                  focus: workspaceCreate.focus
                });
              }}
            >
              <div className="row">
                <input
                  aria-label="New workspace label"
                  value={workspaceCreate.label}
                  onChange={(event) =>
                    setWorkspaceCreate((current) => ({ ...current, label: event.target.value }))
                  }
                  placeholder="New workspace label"
                />
                <input
                  aria-label="New workspace cwd"
                  value={workspaceCreate.cwd}
                  onChange={(event) =>
                    setWorkspaceCreate((current) => ({ ...current, cwd: event.target.value }))
                  }
                  placeholder="cwd"
                />
              </div>
              <label className="check">
                <input
                  type="checkbox"
                  checked={workspaceCreate.focus}
                  onChange={(event) =>
                    setWorkspaceCreate((current) => ({ ...current, focus: event.target.checked }))
                  }
                />
                Focus after create
              </label>
              <button type="submit">Create Workspace</button>
            </form>
          </ActionSection>

          <ActionSection title="Tabs and Agents">
            <div className="row">
              <Select
                label="Tab"
                value={tabs.find((tab) => tab.focused)?.tab_id ?? tabs[0]?.tab_id ?? ""}
                onChange={(tabId) => void runMutation("Tab focus", "tab_focus", { tabId })}
                options={tabs.map((tab) => ({
                  value: tab.tab_id,
                  label: `${tab.label} / ${workspaceLabel(workspaces, tab.workspace_id)}${
                    tab.focused ? " (focused)" : ""
                  }`
                }))}
              />
            </div>
            <div className="row">
              <Select
                label="Agent"
                value={agentSend.target}
                onChange={(target) => setAgentSend((current) => ({ ...current, target }))}
                options={agents.map((agent) => ({
                  value: agent.terminal_id,
                  label: agentLabel(agent)
                }))}
              />
              <button
                type="button"
                disabled={!agentSend.target}
                onClick={() =>
                  void runMutation("Agent focus", "agent_focus", { target: agentSend.target })
                }
              >
                Focus Agent
              </button>
            </div>
            <form
              className="stack"
              onSubmit={(event) => {
                event.preventDefault();
                void runMutation("Agent send", "agent_send", {
                  target: agentSend.target,
                  text: agentSend.text
                });
              }}
            >
              <textarea
                aria-label="Agent text"
                value={agentSend.text}
                onChange={(event) =>
                  setAgentSend((current) => ({ ...current, text: event.target.value }))
                }
                placeholder="Text to send"
                rows={3}
              />
              <button type="submit" disabled={!agentSend.target || !agentSend.text}>
                Send Text
              </button>
            </form>
          </ActionSection>

          <ActionSection title="Kanban">
            <form
              className="stack"
              onSubmit={(event) => {
                event.preventDefault();
                void runMutation("Kanban add", "kanban_add", {
                  title: kanbanAdd.title,
                  description: emptyToUndefined(kanbanAdd.description),
                  status: kanbanAdd.status
                });
                setKanbanAdd((current) => ({ ...current, title: "" }));
              }}
            >
              <div className="row">
                <input
                  aria-label="Kanban title"
                  value={kanbanAdd.title}
                  onChange={(event) =>
                    setKanbanAdd((current) => ({ ...current, title: event.target.value }))
                  }
                  placeholder="Kanban title"
                />
                <StatusSelect
                  value={kanbanAdd.status}
                  onChange={(statusValue) =>
                    setKanbanAdd((current) => ({ ...current, status: statusValue || "todo" }))
                  }
                />
              </div>
              <input
                aria-label="Kanban description path"
                value={kanbanAdd.description}
                onChange={(event) =>
                  setKanbanAdd((current) => ({ ...current, description: event.target.value }))
                }
                placeholder="description path"
              />
              <button type="submit" disabled={!kanbanAdd.title.trim()}>
                Add Item
              </button>
            </form>
            <form
              className="stack"
              onSubmit={(event) => {
                event.preventDefault();
                void runMutation("Kanban update", "kanban_update", {
                  uuid: kanbanUpdate.uuid,
                  title: emptyToUndefined(kanbanUpdate.title),
                  description: emptyToUndefined(kanbanUpdate.description),
                  status: emptyToUndefined(kanbanUpdate.status)
                });
              }}
            >
              <Select
                label="Kanban item"
                value={kanbanUpdate.uuid}
                onChange={(uuid) => {
                  const item = kanbanItems.find((entry) => entry.uuid === uuid);
                  setKanbanUpdate({
                    uuid,
                    title: item?.title ?? "",
                    description: item?.description ?? "",
                    status: item?.status ?? ""
                  });
                }}
                options={kanbanItems.map((item) => ({
                  value: item.uuid,
                  label: `${item.title} (${item.status})`
                }))}
              />
              <div className="row">
                <input
                  aria-label="Update kanban title"
                  value={kanbanUpdate.title}
                  onChange={(event) =>
                    setKanbanUpdate((current) => ({ ...current, title: event.target.value }))
                  }
                  placeholder="Updated title"
                />
                <StatusSelect
                  allowBlank
                  value={kanbanUpdate.status}
                  onChange={(statusValue) =>
                    setKanbanUpdate((current) => ({ ...current, status: statusValue }))
                  }
                />
              </div>
              <input
                aria-label="Update kanban description path"
                value={kanbanUpdate.description}
                onChange={(event) =>
                  setKanbanUpdate((current) => ({ ...current, description: event.target.value }))
                }
                placeholder="updated description path"
              />
              <button type="submit" disabled={!kanbanUpdate.uuid}>
                Update Item
              </button>
            </form>
            <div className="delete-strip">
              <input
                aria-label="Confirm kanban delete uuid"
                value={deleteConfirmUuid}
                onChange={(event) => setDeleteConfirmUuid(event.target.value)}
                placeholder="paste uuid to confirm delete"
              />
              <button
                type="button"
                className="danger"
                disabled={!kanbanUpdate.uuid || deleteConfirmUuid !== kanbanUpdate.uuid}
                onClick={() =>
                  void runMutation("Kanban delete", "kanban_delete", { uuid: kanbanUpdate.uuid })
                }
              >
                Delete Item
              </button>
            </div>
          </ActionSection>
        </section>

        <section className="diagnostics" aria-label="Raw bridge responses">
          {endpoints.map((endpoint) => (
            <ResponsePanel
              key={endpoint.key}
              label={endpoint.label}
              response={results[endpoint.key]}
            />
          ))}
        </section>
      </section>
    </main>
  );
}

function ActionSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <article className="action-section">
      <h2>{title}</h2>
      {children}
    </article>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <article className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </article>
  );
}

function Select({
  label,
  value,
  onChange,
  options
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <label className="select-label">
      <span>{label}</span>
      <select value={value} onChange={(event) => onChange(event.target.value)}>
        {options.length === 0 && <option value="">None</option>}
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function StatusSelect({
  value,
  onChange,
  allowBlank = false
}: {
  value: "" | KanbanStatus;
  onChange: (value: "" | KanbanStatus) => void;
  allowBlank?: boolean;
}) {
  return (
    <select
      aria-label="Kanban status"
      value={value}
      onChange={(event) => onChange(event.target.value as "" | KanbanStatus)}
    >
      {allowBlank && <option value="">No change</option>}
      {KANBAN_STATUSES.map((status) => (
        <option key={status} value={status}>
          {status}
        </option>
      ))}
    </select>
  );
}

function ResponsePanel({
  label,
  response
}: {
  label: string;
  response: BridgeResponse | undefined;
}) {
  return (
    <article className="response-panel">
      <header>
        <h2>{label}</h2>
        <span className={response?.ok ? "badge ok" : "badge error"}>
          {response ? (response.ok ? "ok" : response.error?.kind ?? "error") : "pending"}
        </span>
      </header>
      <pre>{JSON.stringify(response ?? null, null, 2)}</pre>
    </article>
  );
}

function statusLabel(response: BridgeResponse | undefined) {
  if (!response) {
    return "checking";
  }
  if (response.ok) {
    return "server running";
  }
  if (response.error?.kind === "server_not_running") {
    return "server not running";
  }
  return response.error?.kind ?? "error";
}

function countResultArray(response: BridgeResponse | undefined, field: string) {
  return resultArray(response, field).length;
}

function resultArray<T>(response: BridgeResponse | undefined, field: string): T[] {
  const result = responseResult(response);
  const value = result?.[field];
  return Array.isArray(value) ? (value.filter(isRecord) as T[]) : [];
}

function firstPaneId(response: BridgeResponse | undefined) {
  const panes = resultArray<{ pane_id?: unknown }>(response, "panes");
  const first = panes.find((pane) => typeof pane.pane_id === "string");
  return typeof first?.pane_id === "string" ? first.pane_id : undefined;
}

function responseResult(response: BridgeResponse | undefined) {
  if (!response?.ok || !isRecord(response.data)) {
    return undefined;
  }
  return isRecord(response.data.result) ? response.data.result : undefined;
}

function workspaceLabel(workspaces: WorkspaceInfo[], workspaceId: string) {
  return workspaces.find((workspace) => workspace.workspace_id === workspaceId)?.label ?? workspaceId;
}

function agentLabel(agent: AgentInfo) {
  const name = agent.name ?? agent.display_agent ?? agent.agent ?? agent.title ?? agent.terminal_id;
  return `${name} / ${agent.agent_status}${agent.focused ? " (focused)" : ""}`;
}

function emptyToUndefined(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
