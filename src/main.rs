use std::io::Write;
use crate::schedule::Schedule;
use clap::Parser;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::{Context, Editor, Helper, Highlighter, Hinter, Validator};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tabled::settings::Style;
use crate::flight::Flight;
use crate::flight::FlightStatus::{Delayed, Scheduled, Unscheduled};
use crate::time::Time;

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

#[derive(Helper, Hinter, Highlighter, Validator)]
pub struct CompleteHelper {
    pub commands: Vec<String>,
}

impl Completer for CompleteHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, _pos: usize, _ctx: &Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut candidates = Vec::new();

        for cmd in &self.commands {
            if cmd.starts_with(line) {
                candidates.push(Pair {
                    display: cmd.clone(),
                    replacement: format!("{} ", cmd),
                });
            }
        }

        Ok((0, candidates))
    }
}

fn paginate(content: String) {
    let mut pager = Command::new("less")
        .arg("-R")
        .stdin(Stdio::piped())
        .spawn()
        // Fallback to 'more' if 'less' isn't available
        .or_else(|_| Command::new("more").stdin(Stdio::piped()).spawn())
        .expect("Failed to spawn pager");

    let mut stdin = pager.stdin.take().expect("Failed to open stdin for pager");

    if let Err(e) = stdin.write_all(content.as_bytes()) {
        // Broken pipe is common if the user quits the pager early
        if e.kind() != std::io::ErrorKind::BrokenPipe {
            eprintln!("Error writing to pager: {}", e);
        }
    }

    // Wait for the user to close the pager before returning to the ">> " prompt
    let _ = pager.wait();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    println!("Tower online. Loaded flights from {}", args.scenario.display());

    let mut schedule = Schedule::load_from_file(args.scenario.to_str().unwrap())?;
    schedule.assign();

    let config = rustyline::Config::builder()
        .history_ignore_space(true)
        .completion_type(rustyline::CompletionType::List)
        .build();

    let helper = CompleteHelper {
        commands: vec![
            "ls".to_string(),
            "delay".to_string(),
            "curfew".to_string(),
            "recover".to_string(),
            "help".to_string(),
            "exit".to_string(),
        ],
    };

    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(helper));

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
                        let sub = parts.get(1).map(|s| *s).unwrap_or("a");
                        let filtered_flights: Vec<&Flight> = schedule.flights.iter()
                            .filter(|f| match sub {
                                "u" | "unscheduled" => f.status == Unscheduled,
                                "s" | "scheduled"   => f.status == Scheduled || f.status == Delayed,
                                "d" | "delayed" => f.status == Delayed,
                                _ => true, // 'ls' or 'ls a'
                            })
                            .collect();
                        if filtered_flights.is_empty() {
                            println!("No matching flights found.")
                        } else {
                            let mut table = tabled::Table::new(&filtered_flights);
                            table.with(Style::rounded());
                            table.with(tabled::settings::Alignment::left());
                            if filtered_flights.len() > 20 {
                                paginate(table.to_string());
                            } else {
                                println!("{}", table);
                            }
                        }
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
                    "curfew" => {
                        if let (Some(id), Some(from), Some(to)) = (parts.get(1), parts.get(2), parts.get(3)) {
                            let from_u64 = from.parse::<u64>().unwrap_or(0);
                            let to_u64 = to.parse::<u64>().unwrap_or(0);
                            let broken = schedule.apply_curfew(Arc::from(*id), Time(from_u64), Time(to_u64));
                            println!("Applied airport curfew. {} flights became unscheduled.", broken.len());
                        } else {
                            println!("Usage: curfew <airport_id> <minutes> <minutes>");
                        }
                    },
                    "recover" => {
                        schedule.assign();
                        println!("Recovery cycle complete.");
                    },
                    "help" | "?" => {
                        println!("\nAvailable Commands:");
                        println!("  ls [status]         - List all flights in a table or filter by status: u - unscheduled, s - scheduled, d - delayed");
                        println!("  delay <id> <m>      - Inject <m> minutes of delay into flight <id>");
                        println!("  curfew <id> <m> <m> - Inject a curfew from <m> to <m> minutes into airport <id>");
                        println!("  recover             - Re-run assignment to repair unscheduled flights");
                        println!("  help / ?            - Show this help menu");
                        println!("  exit / quit         - Exit the simulator\n");
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
