use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use clap::Parser;
use tabled::settings::Style;
use crate::schedule::Schedule;

mod aircraft;
mod flight;
mod airport;
mod schedule;
mod time;

#[derive(Parser)]
struct Args {
    /// Path to the JSON scenario file
    #[arg(short, long, value_name = "FILE", default_value = "data/default.json")]
    scenario: PathBuf,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    println!("Tower online. Loaded flights from {}", args.scenario.display());

    let mut schedule = Schedule::load_from_file(args.scenario.to_str().unwrap())?;
    schedule.assign();

    loop {
        print!("\n> ");
        io::stdout().flush()?; // Manually flush so the prompt shows up

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() { continue; }

        match parts[0] {
            "ls" => {
                let mut table = tabled::Table::new(&schedule.flights);
                table.with(Style::rounded());
                table.with(tabled::settings::Alignment::left());
                println!("{}", table);
            }
            "delay" => {
                if let (Some(id), Some(mins)) = (parts.get(1), parts.get(2)) {
                    let mins_u64 = mins.parse::<u64>().unwrap_or(0);
                    let broken = schedule.apply_delay(Arc::from(*id), mins_u64);
                    println!("Applied delay. {} flights became unscheduled.", broken.len());
                } else {
                    println!("Usage: delay <flight_id> <minutes>");
                }
            }
            "recover" => {
                schedule.assign();
                println!("Recovery cycle complete.");
            }
            "exit" | "quit" => break,
            _ => println!("Unknown command. Commands: ls, delay, heal, exit"),
        }
    }
    Ok(())
}
