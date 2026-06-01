use crate::api::schema::{EmptyParams, Method, Request};

pub(super) fn run_app_command(args: &[String]) -> std::io::Result<i32> {
    let Some(subcommand) = args.first().map(|arg| arg.as_str()) else {
        print_app_help();
        return Ok(2);
    };

    match subcommand {
        "snapshot" => app_snapshot(&args[1..]),
        "help" | "--help" | "-h" => {
            print_app_help();
            Ok(0)
        }
        _ => {
            print_app_help();
            Ok(2)
        }
    }
}

fn app_snapshot(args: &[String]) -> std::io::Result<i32> {
    if !args.is_empty() {
        eprintln!("usage: herdr app snapshot");
        return Ok(2);
    }

    super::print_response(&super::send_request(&Request {
        id: "cli:app:snapshot".into(),
        method: Method::AppSnapshot(EmptyParams::default()),
    })?)
}

fn print_app_help() {
    eprintln!("herdr app commands:");
    eprintln!("  herdr app snapshot   print a single aggregate app snapshot as JSON");
}
