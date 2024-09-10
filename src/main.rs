use clap::{arg, Command};

use json_stat::parser;
use json_stat::sniffer;
use json_stat::sniffer::print_complex_stats;


fn cli() -> Command {
    Command::new("json-stat")
        .about("Tool for verifying and analyzing JSON")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("check")
                .about("Verifies JSON file(s)")
                .arg(arg!(<JSON>... "Path to JSON file"))
                .arg_required_else_help(true)
        )
        .subcommand(
            Command::new("stat")
                .about("Analyzes JSON file(s)")
                .arg(arg!(<JSON>... "Path to JSON file"))
                .arg_required_else_help(true)
        )
}


fn main() -> Result<(), std::io::Error> {
    let (should_stat, files) = match cli().get_matches().subcommand() {
        Some(("check", sub_matches)) => if let Some(argv) = sub_matches.get_many::<String>("JSON") {
            Ok((false, argv.into_iter().map(String::clone).collect::<Vec<String>>()))
        } else { Err(std::io::Error::from_raw_os_error(22)) },
        Some(("stat", sub_matches)) => if let Some(argv) = sub_matches.get_many::<String>("JSON") {
            Ok((true, argv.into_iter().map(String::clone).collect::<Vec<String>>()))
        } else { Err(std::io::Error::from_raw_os_error(22)) },
        _ => Err(std::io::Error::from_raw_os_error(22))
    }?;

    let mut maybe_stats: Option<sniffer::JsonComplexTypeStats> = None;
    for file in files {
        let maybe_json = match parser::single_json(&file) {
            Ok(maybe_value) => Ok(maybe_value),
            Err(error) => {
                println!("\'{}\' has error at ({}, {}): {}", file, error.row, error.col, error.msg);
                Err(std::io::Error::from_raw_os_error(22))
            }
        }?;

        if let Some(json) = maybe_json {
            println!("{} is valid JSON", file);
            if should_stat {
                maybe_stats = Some(match maybe_stats {
                    Some(prev) => prev.merge_stats(json),
                    None => sniffer::JsonComplexTypeStats::from_json(json)
                });
            }
            continue;
        }
        println!("{} is not valid JSON - SKIP", file);
    }

    if should_stat {
        if let Some(stats) = maybe_stats {
            print_complex_stats(stats);
        } else {
            println!("No stat information collected - SKIP");
        }
    }
    Ok(())
}
