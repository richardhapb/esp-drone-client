use std::env::args;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Udp(String),
    Help(String),
}

impl Command {
    pub fn from_args() -> Result<Command, String> {
        Self::parse(args())
    }

    fn parse(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
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
    println!("UDP command: run the client over UDP");
    println!();
    println!("-H, --host    Drone's host. e.g. 192.168.1.12");
    println!("-p, --port    Drone's port. e.g. 2390");
    println!("-h, --help    This help");
    println!();
}

fn resolve_udp_args(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
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
    help.push_str("udp           Run the client over UDP\n");
    help.push_str("-h, --help    This help\n");
    help
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, String> {
        Command::parse(args.iter().map(|a| a.to_string()))
    }

    #[test]
    fn udp_with_default_port() {
        assert_eq!(
            parse(&["bin", "udp", "-H", "192.168.1.12"]),
            Ok(Command::Udp("192.168.1.12:2390".into()))
        );
    }

    #[test]
    fn udp_with_custom_port_and_long_flags() {
        assert_eq!(
            parse(&["bin", "UDP", "--host", "10.0.0.1", "--port", "4000"]),
            Ok(Command::Udp("10.0.0.1:4000".into()))
        );
    }

    #[test]
    fn udp_requires_host() {
        assert_eq!(parse(&["bin", "udp"]), Err("host is required".into()));
    }

    #[test]
    fn udp_flag_without_value() {
        assert_eq!(parse(&["bin", "udp", "-H"]), Err("no host provided".into()));
        assert_eq!(
            parse(&["bin", "udp", "-H", "h", "-p"]),
            Err("no port provided".into())
        );
    }

    #[test]
    fn udp_invalid_argument() {
        assert_eq!(
            parse(&["bin", "udp", "--nope"]),
            Err("Invalid argument --nope".into())
        );
    }

    #[test]
    fn help_flags() {
        for flag in ["--help", "-h"] {
            assert!(matches!(parse(&["bin", flag]), Ok(Command::Help(_))));
        }
    }

    #[test]
    fn command_is_required() {
        assert_eq!(parse(&["bin"]), Err("command is required".into()));
        assert_eq!(parse(&["bin", "bogus"]), Err("command is required".into()));
    }
}
