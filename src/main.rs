use crate::flight::Flight;
use crate::flight::FlightStatus::{Delayed, Scheduled, Unscheduled};
use crate::flight::UnscheduledReason::*;
use crate::schedule::{DisruptionType, Schedule};
use crate::time::Time;
use clap::Parser;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::{Context, Editor, Helper, Highlighter, Hinter, Validator};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tabled::settings::Style;

mod aircraft;
mod airport;
mod flight;
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

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
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
    println!(
        "Tower online. Loaded flights from {}",
        args.scenario.display()
    );

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
            "explain".to_string(),
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
                if trimmed.is_empty() {
                    continue;
                }

                rl.add_history_entry(trimmed)?;

                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                match parts[0] {
                    "ls" => {
                        let mut day = None;
                        let mut status = None;
                        for part in parts.iter().skip(1) {
                            if let Ok(d) = part.parse::<u64>() {
                                if d > 0 {
                                    day = Some(d);
                                }
                            } else {
                                status = match *part {
                                    "u" | "unscheduled" => Some(Unscheduled(Waiting)),
                                    "s" | "scheduled" => Some(Scheduled),
                                    "d" | "delayed" => Some(Delayed),
                                    _ => None,
                                }
                            }
                        }
                        let filtered_flights: Vec<&Flight> = schedule
                            .flights
                            .iter()
                            .filter(|f| {
                                if let Some(d) = day {
                                    f.departure_time / Time(1440) == Time(d - 1)
                                } else {
                                    true
                                }
                            })
                            .filter(|f| {
                                if let Some(s) = &status {
                                    f.status == *s
                                } else {
                                    true
                                }
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
                    }
                    "delay" => {
                        if let (Some(id), Some(mins)) = (parts.get(1), parts.get(2)) {
                            let mins_u64 = mins.parse::<u64>().unwrap_or(0);
                            schedule.apply_delay(Arc::from(*id), mins_u64);
                            let report = schedule.last_report().unwrap();
                            println!(
                                "\nFlight {} delayed by {} min\n\nImpact:\n  Delayed: {} flight{}\n  Unscheduled: {} flight{}\n\nFirst break:\n  {}\n",
                                *id,
                                mins_u64,
                                report.affected.len(),
                                if report.affected.len() == 1 { "" } else { "s " },
                                report.unscheduled.len(),
                                if report.unscheduled.len() == 1 {
                                    ""
                                } else {
                                    "s "
                                },
                                match &report.first_break {
                                    None => "None".to_string(),
                                    Some((flight_id, reason)) =>
                                        format!("{} ({:?})", flight_id, reason),
                                }
                            );
                        } else {
                            println!("Usage: delay <flight_id> <minutes>");
                        }
                    }
                    "curfew" => {
                        if let (Some(id), Some(from), Some(to)) =
                            (parts.get(1), parts.get(2), parts.get(3))
                        {
                            let from_u64 = from.parse::<u64>().unwrap_or(0);
                            let to_u64 = to.parse::<u64>().unwrap_or(0);
                            schedule.apply_curfew(Arc::from(*id), Time(from_u64), Time(to_u64));
                            let report = schedule.last_report().unwrap();
                            println!(
                                "\nCurfew applied at {} ({} - {})\n\nImpact:\n  Unscheduled: {} flight{}\n\nFirst break:\n  {}\n",
                                *id,
                                Time(from_u64),
                                Time(to_u64),
                                report.unscheduled.len(),
                                if report.unscheduled.len() == 1 {
                                    ""
                                } else {
                                    "s "
                                },
                                match &report.first_break {
                                    None => "None".to_string(),
                                    Some((flight_id, reason)) =>
                                        format!("{} ({:?})", flight_id, reason),
                                },
                            );
                        } else {
                            println!("Usage: curfew <airport_id> <minutes> <minutes>");
                        }
                    }
                    "explain" => {
                        if let Some(report) = schedule.last_report() {
                            let trigger = match &report.kind {
                                DisruptionType::Delay { flight, delay_by } => {
                                    format!("Flight {flight} delayed by {delay_by} min")
                                }
                                DisruptionType::Curfew { airport, from, to } => {
                                    format!("Curfew applied at {airport} ({from} - {to})")
                                }
                            };
                            if parts.get(1) == Some(&"full") {
                                let impact = match &report.kind {
                                    DisruptionType::Delay { .. } if report.affected.len() > 0 => {
                                        &format!(
                                            "\n\nDelayed flights ({}):{}",
                                            report.affected.len(),
                                            report
                                                .affected
                                                .iter()
                                                .map(|f| format!("\n  {f}"))
                                                .collect::<String>()
                                        )
                                    }
                                    DisruptionType::Delay { .. } => "\n\nDelayed flights:\n  None",
                                    DisruptionType::Curfew { .. } => "",
                                };
                                println!(
                                    "\nExplain (last disruption)\n\nTrigger:\n  {}{}{}\n",
                                    trigger,
                                    impact,
                                    if report.unscheduled.len() == 0 {
                                        "\n\nUnscheduled:\n  None".to_string()
                                    } else {
                                        format!(
                                            "\n\nUnscheduled flights ({}):{}",
                                            report.unscheduled.len(),
                                            report
                                                .unscheduled
                                                .iter()
                                                .map(|(fid, reason)| format!(
                                                    "\n  {fid} ({:?})",
                                                    reason
                                                ))
                                                .collect::<String>()
                                        )
                                    },
                                );
                            } else {
                                let impact = match &report.kind {
                                    DisruptionType::Delay { .. } => &format!(
                                        "\n  Delayed: {} flight{}",
                                        report.affected.len(),
                                        if report.affected.len() == 1 { "" } else { "s" }
                                    ),
                                    DisruptionType::Curfew { .. } => "",
                                };
                                println!(
                                    "\nExplain (last disruption)\n\nTrigger:\n  {}\n\nImpact:{}\n  Unscheduled: {} flight{}\n\nFirst break:\n  {}\n",
                                    trigger,
                                    impact,
                                    report.unscheduled.len(),
                                    if report.unscheduled.len() == 1 {
                                        ""
                                    } else {
                                        "s "
                                    },
                                    match &report.first_break {
                                        None => "None".to_string(),
                                        Some((flight_id, reason)) =>
                                            format!("{} ({:?})", flight_id, reason),
                                    }
                                );
                            }
                        } else {
                            println!("No report to explain");
                        }
                    }
                    "recover" => {
                        schedule.assign();
                        println!("Recovery cycle complete.");
                    }
                    "stats" => {
                        let mut s = 0;
                        let mut d = 0;
                        let mut uw = 0;
                        let mut umde = 0;
                        let mut uam = 0;
                        let mut uac = 0;
                        let mut ubc = 0;
                        let total = schedule.flights.len();

                        for f in &schedule.flights {
                            match f.status {
                                Scheduled => s += 1,
                                Delayed => d += 1,
                                Unscheduled(Waiting) => uw += 1,
                                Unscheduled(MaxDelayExceeded) => umde += 1,
                                Unscheduled(AirportCurfew) => uac += 1,
                                Unscheduled(AircraftMaintenance) => uam += 1,
                                Unscheduled(BrokenChain) => ubc += 1,
                            }
                        }

                        println!("\nFleet Utilization Summary:");
                        println!("---------------------------");
                        println!(
                            "Scheduled:                          {} ({:.1}%)",
                            s,
                            (s as f64 / total as f64) * 100.0
                        );
                        println!(
                            "Delayed:                            {} ({:.1}%)",
                            d,
                            (d as f64 / total as f64) * 100.0
                        );
                        println!(
                            "Unscheduled (Waiting):              {} ({:.1}%)",
                            uw,
                            (uw as f64 / total as f64) * 100.0
                        );
                        println!(
                            "Unscheduled (Max Delay Exceeded):   {} ({:.1}%)",
                            umde,
                            (umde as f64 / total as f64) * 100.0
                        );
                        println!(
                            "Unscheduled (Airport Curfew):       {} ({:.1}%)",
                            uac,
                            (uac as f64 / total as f64) * 100.0
                        );
                        println!(
                            "Unscheduled (Aircraft Maintenance): {} ({:.1}%)",
                            uam,
                            (uam as f64 / total as f64) * 100.0
                        );
                        println!(
                            "Unscheduled (Broken Chain):         {} ({:.1}%)",
                            ubc,
                            (ubc as f64 / total as f64) * 100.0
                        );
                        println!("---------------------------");
                        println!("Total Flights: {}\n", total);
                    }
                    "help" | "?" => {
                        println!("\nAvailable Commands:");
                        println!(
                            "  ls [status]         - List all flights in a table or filter by status: u - unscheduled, s - scheduled, d - delayed"
                        );
                        println!(
                            "  delay <id> <m>      - Inject <m> minutes of delay into flight <id>"
                        );
                        println!(
                            "  curfew <id> <m> <m> - Inject a curfew from <m> to <m> minutes into airport <id>"
                        );
                        println!(
                            "  explain [full]      - Explain the most recent disruption (use 'full' for full causal trace)"
                        );
                        println!(
                            "  recover             - Re-run assignment to repair unscheduled flights"
                        );
                        println!("  stats               - Display summary statistics");
                        println!("  help / ?            - Show this help menu");
                        println!("  exit / quit         - Exit the simulator\n");
                    }
                    "exit" | "quit" => break,
                    _ => println!("Unknown command: {}", parts[0]),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
