use crate::{
    cli::Command,
    commander::Commander,
    link::UdpLink,
    logging::Toc,
    packet::{Port, channels},
};

mod cli;
mod commander;
mod link;
mod logging;
mod packet;

fn main() -> Result<(), String> {
    let command = Command::from_args()?;

    let link = match command {
        Command::Udp(address) => UdpLink::connect(&address)?,
        Command::Help(help) => {
            println!("{}", help);
            std::process::exit(0);
        }
    };

    let thrust = 10000;

    let toc = Toc::get_info_v2(&link)?;
    println!("{}", toc);
    println!();

    let bat = toc.get_item(&link, "pm.vbat");
    if let Some(bat) = bat {
        println!("{}", bat);
    }

    println!("Ramping...");
    for _ in 1..30 {
        let cmd = Commander::new(0f32, 0f32, 0f32, thrust);
        cmd.send(&link)?;
        sleep(100);
    }

    let zero_cmd = Commander::default();

    // Cool down
    println!("Cooling down...");
    for _ in 1..30 {
        zero_cmd.send(&link)?;
        sleep(10);
    }

    Ok(())
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}
