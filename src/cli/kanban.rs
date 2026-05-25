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
    eprintln!("  herdr kanban add <title> [--description <desc>] [--status <todo|in-progress|need-review|done>]");
    eprintln!("  herdr kanban move <uuid> <status>");
    eprintln!("  herdr kanban list [--status <todo|in-progress|need-review|done>]");
    eprintln!(
        "  herdr kanban update <uuid> [--title <title>] [--description <desc>] [--status <status>]"
    );
    eprintln!("  herdr kanban delete <uuid>");
}

fn kanban_add(args: &[String]) -> std::io::Result<i32> {
    let Some(title) = args.first() else {
        eprintln!("usage: herdr kanban add <title> [--description <desc>] [--status <todo|in-progress|need-review|done>]");
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
                description = Some(val.clone());
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
        }),
    })?)
}

fn kanban_move(args: &[String]) -> std::io::Result<i32> {
    let Some(uuid) = args.first() else {
        eprintln!("usage: herdr kanban move <uuid> <status>");
        return Ok(2);
    };
    let Some(status_str) = args.get(1) else {
        eprintln!("usage: herdr kanban move <uuid> <status>");
        return Ok(2);
    };
    if args.len() != 2 {
        eprintln!("usage: herdr kanban move <uuid> <status>");
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
        eprintln!("usage: herdr kanban update <uuid> [--title <title>] [--description <desc>] [--status <status>]");
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
                description = Some(val.clone());
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
