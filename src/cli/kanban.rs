use crate::api::schema::{
    KanbanAddParams, KanbanDeleteParams, KanbanListParams, KanbanStatus, KanbanUpdateParams,
    Method, Request,
};

pub(super) fn run_kanban_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_kanban_help();
        return Ok(2);
    };

    match subcommand {
        "add" => kanban_add(&args[1..]),
        "move" => kanban_move(&args[1..]),
        "list" => kanban_list(&args[1..]),
        "update" => kanban_update(&args[1..]),
        "delete" => kanban_delete(&args[1..]),
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
    eprintln!("  herdr kanban add <title> [--description <path.md>] [--status <todo|in-progress|need-review|done>] [--detached]");
    eprintln!("  herdr kanban move <uuid> <status> [--detached]");
    eprintln!("  herdr kanban list [--status <todo|in-progress|need-review|done>]");
    eprintln!(
        "  herdr kanban update <uuid> [--title <title>] [--description <path.md>] [--status <status>] [--detached]"
    );
    eprintln!("    updates task details; running with only <uuid> (and without --detached) updates the terminal tracking to the current terminal and lists the current status");
    eprintln!("  herdr kanban delete <uuid>");
}

fn kanban_add(args: &[String]) -> std::io::Result<i32> {
    let Some(title) = args.first() else {
        eprintln!("usage: herdr kanban add <title> [--description <path.md>] [--status <todo|in-progress|need-review|done>] [--detached]");
        return Ok(2);
    };

    let mut description = None;
    let mut status = None;
    let mut _detached = false;
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
                        eprintln!(
                            "invalid status: {val} (expected todo|in-progress|need-review|done)"
                        );
                        return Ok(2);
                    }
                };
                status = Some(parsed_status);
                index += 2;
            }
            "--detached" => {
                _detached = true;
                index += 1;
            }
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    super::print_response(&super::send_request(&Request {
        id: "cli:kanban:add".into(),
        method: Method::KanbanAdd(KanbanAddParams {
            title: title.clone(),
            description,
            status,
            terminal_id: None,
        }),
    })?)
}

fn kanban_move(args: &[String]) -> std::io::Result<i32> {
    let detached = args.iter().any(|arg| arg == "--detached");
    let clean_args: Vec<String> = args
        .iter()
        .filter(|arg| arg.as_str() != "--detached")
        .cloned()
        .collect();

    let Some(uuid) = clean_args.first() else {
        eprintln!("usage: herdr kanban move <uuid> <status> [--detached]");
        return Ok(2);
    };
    let Some(status_str) = clean_args.get(1) else {
        eprintln!("usage: herdr kanban move <uuid> <status> [--detached]");
        return Ok(2);
    };
    if clean_args.len() != 2 {
        eprintln!("usage: herdr kanban move <uuid> <status> [--detached]");
        return Ok(2);
    }

    let status = match KanbanStatus::from_str(status_str) {
        Some(s) => s,
        None => {
            eprintln!("invalid status: {status_str} (expected todo|in-progress|need-review|done)");
            return Ok(2);
        }
    };

    super::print_response(&super::send_request(&Request {
        id: "cli:kanban:move".into(),
        method: Method::KanbanUpdate(KanbanUpdateParams {
            uuid: uuid.clone(),
            title: None,
            description: None,
            status: Some(status),
            terminal_id: if detached {
                None
            } else {
                std::env::var("HERDR_PANE_ID").ok()
            },
        }),
    })?)
}

fn kanban_list(args: &[String]) -> std::io::Result<i32> {
    let mut status = None;
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
                        eprintln!(
                            "invalid status: {val} (expected todo|in-progress|need-review|done)"
                        );
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

    super::print_response(&super::send_request(&Request {
        id: "cli:kanban:list".into(),
        method: Method::KanbanList(KanbanListParams { status }),
    })?)
}

fn kanban_update(args: &[String]) -> std::io::Result<i32> {
    let Some(uuid) = args.first() else {
        eprintln!("usage: herdr kanban update <uuid> [--title <title>] [--description <path.md>] [--status <status>] [--detached]");
        return Ok(2);
    };

    let mut title = None;
    let mut description = None;
    let mut status = None;
    let mut detached = false;
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
                        eprintln!(
                            "invalid status: {val} (expected todo|in-progress|need-review|done)"
                        );
                        return Ok(2);
                    }
                };
                status = Some(parsed_status);
                index += 2;
            }
            "--detached" => {
                detached = true;
                index += 1;
            }
            other => {
                eprintln!("unknown option: {other}");
                return Ok(2);
            }
        }
    }

    super::print_response(&super::send_request(&Request {
        id: "cli:kanban:update".into(),
        method: Method::KanbanUpdate(KanbanUpdateParams {
            uuid: uuid.clone(),
            title,
            description,
            status,
            terminal_id: if detached {
                None
            } else {
                std::env::var("HERDR_PANE_ID").ok()
            },
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

    super::print_response(&super::send_request(&Request {
        id: "cli:kanban:delete".into(),
        method: Method::KanbanDelete(KanbanDeleteParams { uuid: uuid.clone() }),
    })?)
}
