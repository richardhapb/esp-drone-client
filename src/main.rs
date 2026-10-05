use crate::{
    cli::Command,
    commander::Commander,
    link::{Link, UdpLink},
    logging::{LogData, Logging},
    packet::{Crtp, Port, channels},
};

mod cli;
mod commander;
mod errno;
mod link;
mod logging;
mod packet;

fn main() -> Result<(), String> {
    let command = Command::from_args()?;

    let l = match command {
        Command::Udp(address) => UdpLink::connect(&address)?,
        Command::Help(help) => {
            println!("{}", help);
            std::process::exit(0);
        }
    };
    let log = Logging::new(&l);

    let thrust = 10000;

    let toc = log.get_info_v2()?;
    println!("{}", toc);
    println!();

    log.reset()?;
    let mut vars = Vec::new();
    if let Some(bat) = toc.get_item(&log, "pm.vbat") {
        println!("{}", bat);
        log.create_block(7, &[&bat])?;
        log.start_block(7, std::time::Duration::from_millis(100))?;
        vars.push(bat);
    }

    println!("Ramping...");
    for _ in 1..30 {
        let cmd = Commander::new(0f32, 0f32, 0f32, thrust);
        cmd.send(&l)?;
        let res = l.recv_with_retries()?;
        match LogData::from_raw(&res) {
            Ok(sample) => println!("{:?}", sample.values(&vars.iter().collect::<Vec<_>>())?),
            Err(_) => println!("{}", Crtp::from_raw(&res)?),
        }
        sleep(100);
    }

    if !vars.is_empty() {
        log.stop_block(7)?;
    }

    let zero_cmd = Commander::default();

    // Cool down
    println!("Cooling down...");
    for _ in 1..30 {
        zero_cmd.send(&l)?;
        sleep(10);
    }

    Ok(())
}

fn sleep(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}
