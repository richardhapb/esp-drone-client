use std::env::{Args, args};

pub enum Command {
    Udp(String),
    Help(String),
}

impl Command {
    pub fn from_args() -> Result<Command, String> {
        let mut args = args();
        // First argument has the process name
        args.next();

        if let Some(first) = args.next() {
            if first.to_lowercase() == "udp" {
                return Ok(Command::Udp(resolve_udp_args(&mut args).inspect_err(
                    |_| {
                        print_udp_usage();
                    },
                )?));
            }

            if ["--help", "-h"].contains(&first.as_str()) {
                return Ok(Command::Help(resolve_help()));
            }
        }

        Err("command is required".into())
    }
}

pub fn print_udp_usage() {
    println!("UDP command: run the client over UDP transport");
    println!();
    println!("-H, --host    Drone's host. e.g. 192.168.1.12");
    println!("-p, --port    Drone's port. e.g. 2390");
    println!("-h, --help    This help");
    println!();
}

fn resolve_udp_args(args: &mut Args) -> Result<String, String> {
    let mut port = String::from("2390");
    let mut host = String::new();

    while let Some(a) = args.next() {
        if a == "--host" || a == "-H" {
            host = args.next().ok_or("no host provided")?;
            continue;
        }

        if a == "--port" || a == "-p" {
            port = args.next().ok_or("no port provided")?;
            continue;
        }

        if a == "--help" || a == "-h" {
            print_udp_usage();
            std::process::exit(0);
        }

        return Err(format!("Invalid argument {a}"));
    }

    if host.is_empty() {
        return Err("host is required".into());
    }

    Ok(format!("{host}:{port}"))
}

fn resolve_help() -> String {
    let mut help = String::new();
    help.push_str("esp-drone-client: Control ESP32 drone from cli\n");
    help.push('\n');
    help.push_str("udp           Run the client over UDP transport\n");
    help.push_str("-h, --help    This help\n");
    help
}
