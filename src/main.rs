use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use clap::Parser;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("Tower online. Loaded flights from {}", args.scenario.display());

    let mut schedule = Schedule::load_from_file(args.scenario.to_str().unwrap())?;
    schedule.assign();

    let mut rl = DefaultEditor::new()?;


    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }

                rl.add_history_entry(trimmed)?;

                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                match parts[0] {
                    "ls" => {
                        let mut table = tabled::Table::new(&schedule.flights);
                        table.with(Style::rounded());
                        table.with(tabled::settings::Alignment::left());
                        println!("{}", table);
                    },
                    "delay" => {
                        if let (Some(id), Some(mins)) = (parts.get(1), parts.get(2)) {
                            let mins_u64 = mins.parse::<u64>().unwrap_or(0);
                            let broken = schedule.apply_delay(Arc::from(*id), mins_u64);
                            println!("Applied delay. {} flights became unscheduled.", broken.len());
                        } else {
                            println!("Usage: delay <flight_id> <minutes>");
                        }
                    },
                    "recover" => {
                        schedule.assign();
                        println!("Recovery cycle complete.");
                    },
                    "help" | "?" => {
                        println!("\nAvailable Commands:");
                        println!("  ls             - List all flights in a table");
                        println!("  delay <id> <m> - Inject <m> minutes of delay into flight <id>");
                        println!("  recover        - Re-run assignment to repair unscheduled flights");
                        println!("  help / ?       - Show this help menu");
                        println!("  exit / quit    - Exit the simulator\n");
                    },
                    "exit" | "quit" => break,
                    _ => println!("Unknown command: {}", parts[0]),
                }
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
