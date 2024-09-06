use clap::{arg, Command};

use json_stat::parser;


fn cli() -> Command {
    Command::new("json-stat")
        .about("Tool for verifying and analyzing JSON")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("check")
                .about("Verifies JSON file(s)")
                .arg(arg!(<JSON> "Path to JSON file"))
                .arg_required_else_help(true)
        )
}


fn _main(files: &[String]) -> Result<(), std::io::Error> {
    for file in files {
        match parser::single_json(file) {
            Ok(maybe_value) => match maybe_value {
                Some(value) => println!("{} is valid JSON: {:?}", file, value),
                None => println!("{} is not valid JSON", file)
            },
            Err(error) => println!("{} has error at ({}, {}): {}",
                file, error.row, error.col, error.msg)
        }
    }
    Ok(())
}


fn main() -> Result<(), std::io::Error> {
    let m = cli().get_matches();
    match m.subcommand() {
        Some(("check", sub_matches)) => if let Some(arg) = sub_matches.get_one::<String>("JSON") {
            let argv = [arg.clone()];
            _main(&argv)
        } else {
            Err(std::io::Error::from_raw_os_error(22))
        },
        _ => unreachable!()
    }
}
