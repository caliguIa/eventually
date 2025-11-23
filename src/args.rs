use std::env;
use std::io::Result;

use crate::launchd::{Service, ID};

pub fn handle_args() -> Option<Result<()>> {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        return None;
    }

    match args[1].as_str() {
        "service" => {
            if args.len() < 3 {
                eprintln!("Usage: eventually service <install|uninstall|start|stop|restart>");
                std::process::exit(1);
            }

            let service = match Service::try_new(ID) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to initialize service: {e}");
                    std::process::exit(1);
                }
            };

            let result = match args[2].as_str() {
                "install" => service.install(),
                "uninstall" => service.uninstall(),
                "start" => service.start(),
                "stop" => service.stop(),
                "restart" => service.restart(),
                cmd => {
                    eprintln!("Unknown service command: {cmd}");
                    eprintln!("Available commands: install, uninstall, start, stop, restart");
                    std::process::exit(1);
                }
            };

            Some(result)
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}
