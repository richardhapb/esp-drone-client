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

const THRUST_START: u16 = 20000;
const THRUST_MAX: u16 = 25000;
const THRUST_STEP: u16 = 500;

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

    // Zero thrust setpoint releases the firmware thrust lock
    Commander::default().send(&l)?;

    println!("Ramping...");
    for thrust in ramp(THRUST_START, THRUST_MAX, THRUST_STEP) {
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

fn ramp(start: u16, max: u16, step: u16) -> impl Iterator<Item = u16> {
    (start..=max).step_by(step as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_is_inclusive_and_stepped() {
        let values: Vec<u16> = ramp(20000, 21000, 500).collect();
        assert_eq!(values, [20000, 20500, 21000]);
    }

    #[test]
    fn ramp_covers_configured_range() {
        let values: Vec<u16> = ramp(THRUST_START, THRUST_MAX, THRUST_STEP).collect();
        assert_eq!(values.first(), Some(&THRUST_START));
        assert_eq!(values.last(), Some(&THRUST_MAX));
    }
}
