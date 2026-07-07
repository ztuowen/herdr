use crate::api::schema::{
    KanbanAddParams, KanbanDeleteParams, KanbanListParams, KanbanStatus, KanbanUpdateParams,
    Method, Request,
};

const STATUS_HELP: &str = "todo|ongoing|blocked|reviewing|done";

pub(crate) fn run_kanban_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_kanban_help();
        return Ok(2);
    };

    match subcommand {
        "add" => kanban_add(&args[1..]),
        "list" => kanban_list(&args[1..]),
        "update" => kanban_update(&args[1..]),
        "delete" => kanban_delete(&args[1..]),
        "attach" => kanban_attach(&args[1..]),
        "detach" => kanban_detach(&args[1..]),
        "help" | "--help" | "-h" => {
            print_kanban_help();
            Ok(0)
        }
        _ => {
            print_kanban_help();
            Ok(2)
        }
    }
}

fn print_kanban_help() {
    eprintln!("herdr kanban commands:");
    eprintln!("  herdr kanban add <title> [--description <path.md>] [--status <{STATUS_HELP}>]");
    eprintln!("  herdr kanban list [--status <{STATUS_HELP}>] [--pane]");
    eprintln!(
        "  herdr kanban update <uuid> [--title <title>] [--description <path.md>] [--status <status>]"
    );
    eprintln!("  herdr kanban delete <uuid>");
    eprintln!("  herdr kanban attach <uuid>");
    eprintln!("  herdr kanban detach <uuid>");
}

fn kanban_add(args: &[String]) -> std::io::Result<i32> {
    let Some(title) = args.first() else {
        eprintln!(
            "usage: herdr kanban add <title> [--description <path.md>] [--status <{STATUS_HELP}>]"
        );
        return Ok(2);
    };

    let mut description = None;
    let mut status = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--description" => {
                let Some(val) = args.get(index + 1) else {
                    eprintln!("missing value for --description");
                    return Ok(2);
                };
                if val.is_empty() {
                    description = Some(String::new());
                } else {
                    let path = std::path::Path::new(val);
                    let abs_path = std::fs::canonicalize(path)
                        .or_else(|_| std::env::current_dir().map(|cwd| cwd.join(path)))
                        .unwrap_or_else(|_| path.to_path_buf());
                    description = Some(abs_path.to_string_lossy().to_string());
                }
                index += 2;
            }
            "--status" => {
                let Some(val) = args.get(index + 1) else {
                    eprintln!("missing value for --status");
                    return Ok(2);
                };
                let parsed_status = match KanbanStatus::from_str(val) {
                    Some(s) => s,
                    None => {
                        eprintln!("invalid status: {val} (expected {STATUS_HELP})");
                        return Ok(2);
                    }
                };
                status = Some(parsed_status);
                index += 2;
            }
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    crate::cli::print_response(&crate::cli::send_request(&Request {
        id: "cli:kanban:add".into(),
        method: Method::KanbanAdd(KanbanAddParams {
            title: title.clone(),
            description,
            status,
            terminal_id: None,
        }),
    })?)
}

fn kanban_list(args: &[String]) -> std::io::Result<i32> {
    let mut status = None;
    let mut pane = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--status" => {
                let Some(val) = args.get(index + 1) else {
                    eprintln!("missing value for --status");
                    return Ok(2);
                };
                let parsed_status = match KanbanStatus::from_str(val) {
                    Some(s) => s,
                    None => {
                        eprintln!("invalid status: {val} (expected {STATUS_HELP})");
                        return Ok(2);
                    }
                };
                status = Some(parsed_status);
                index += 2;
            }
            "--pane" => {
                pane = true;
                index += 1;
            }
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    let terminal_id = if pane {
        let Some(tid) = std::env::var("HERDR_PANE_ID").ok() else {
            eprintln!("error: HERDR_PANE_ID environment variable is not set");
            return Ok(1);
        };
        Some(tid)
    } else {
        None
    };

    crate::cli::print_response(&crate::cli::send_request(&Request {
        id: "cli:kanban:list".into(),
        method: Method::KanbanList(KanbanListParams {
            status,
            terminal_id,
        }),
    })?)
}

fn kanban_update(args: &[String]) -> std::io::Result<i32> {
    let Some(uuid) = args.first() else {
        eprintln!("usage: herdr kanban update <uuid> [--title <title>] [--description <path.md>] [--status <status>]");
        return Ok(2);
    };

    let mut title = None;
    let mut description = None;
    let mut status = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--title" => {
                let Some(val) = args.get(index + 1) else {
                    eprintln!("missing value for --title");
                    return Ok(2);
                };
                title = Some(val.clone());
                index += 2;
            }
            "--description" => {
                let Some(val) = args.get(index + 1) else {
                    eprintln!("missing value for --description");
                    return Ok(2);
                };
                if val.is_empty() {
                    description = Some(String::new());
                } else {
                    let path = std::path::Path::new(val);
                    let abs_path = std::fs::canonicalize(path)
                        .or_else(|_| std::env::current_dir().map(|cwd| cwd.join(path)))
                        .unwrap_or_else(|_| path.to_path_buf());
                    description = Some(abs_path.to_string_lossy().to_string());
                }
                index += 2;
            }
            "--status" => {
                let Some(val) = args.get(index + 1) else {
                    eprintln!("missing value for --status");
                    return Ok(2);
                };
                let parsed_status = match KanbanStatus::from_str(val) {
                    Some(s) => s,
                    None => {
                        eprintln!("invalid status: {val} (expected {STATUS_HELP})");
                        return Ok(2);
                    }
                };
                status = Some(parsed_status);
                index += 2;
            }
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    crate::cli::print_response(&crate::cli::send_request(&Request {
        id: "cli:kanban:update".into(),
        method: Method::KanbanUpdate(KanbanUpdateParams {
            uuid: uuid.clone(),
            title,
            description,
            status,
            terminal_id: None,
            clear_terminal_id: None,
        }),
    })?)
}

fn kanban_delete(args: &[String]) -> std::io::Result<i32> {
    let Some(uuid) = args.first() else {
        eprintln!("usage: herdr kanban delete <uuid>");
        return Ok(2);
    };
    if args.len() != 1 {
        eprintln!("usage: herdr kanban delete <uuid>");
        return Ok(2);
    }

    crate::cli::print_response(&crate::cli::send_request(&Request {
        id: "cli:kanban:delete".into(),
        method: Method::KanbanDelete(KanbanDeleteParams { uuid: uuid.clone() }),
    })?)
}

fn kanban_attach(args: &[String]) -> std::io::Result<i32> {
    let Some(uuid) = args.first() else {
        eprintln!("usage: herdr kanban attach <uuid>");
        return Ok(2);
    };
    if args.len() != 1 {
        eprintln!("usage: herdr kanban attach <uuid>");
        return Ok(2);
    }

    let Some(terminal_id) = std::env::var("HERDR_PANE_ID").ok() else {
        eprintln!("error: HERDR_PANE_ID environment variable is not set");
        return Ok(1);
    };

    crate::cli::print_response(&crate::cli::send_request(&Request {
        id: "cli:kanban:attach".into(),
        method: Method::KanbanUpdate(KanbanUpdateParams {
            uuid: uuid.clone(),
            title: None,
            description: None,
            status: None,
            terminal_id: Some(terminal_id),
            clear_terminal_id: None,
        }),
    })?)
}

fn kanban_detach(args: &[String]) -> std::io::Result<i32> {
    let Some(uuid) = args.first() else {
        eprintln!("usage: herdr kanban detach <uuid>");
        return Ok(2);
    };
    if args.len() != 1 {
        eprintln!("usage: herdr kanban detach <uuid>");
        return Ok(2);
    }

    crate::cli::print_response(&crate::cli::send_request(&Request {
        id: "cli:kanban:detach".into(),
        method: Method::KanbanUpdate(KanbanUpdateParams {
            uuid: uuid.clone(),
            title: None,
            description: None,
            status: None,
            terminal_id: None,
            clear_terminal_id: Some(true),
        }),
    })?)
}
