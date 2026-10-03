use crate::{
    cli::Command,
    commander::Commander,
    logging::Toc,
    packet::{Port, channels},
    transport::UdpTransport,
};

mod cli;
mod commander;
mod logging;
mod packet;
mod transport;

fn main() -> Result<(), String> {
    let command = Command::from_args()?;

    let transport = match command {
        Command::Udp(address) => UdpTransport::connect(&address)?,
        Command::Help(help) => {
            println!("{}", help);
            std::process::exit(0);
        }
    };

    let thrust = 10000;

    let toc = Toc::get_info_v2(&transport)?;
    println!("{}", toc);
    println!();

    let bat = toc.get_item(&transport, "pm.vbat");
    if let Some(bat) = bat {
        println!("{}", bat);
    }

    println!("Ramping...");
    for _ in 1..30 {
        let cmd = Commander::new(0f32, 0f32, 0f32, thrust);
        cmd.send(&transport)?;
        sleep(100);
    }

    let zero_cmd = Commander::default();

    // Cool down
    println!("Cooling down...");
    for _ in 1..30 {
        zero_cmd.send(&transport)?;
        sleep(10);
    }

    Ok(())
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}
