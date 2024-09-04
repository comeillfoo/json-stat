use std::env;
use json_stat::parser;


fn usage(arg0: &String) -> Result<(), std::io::Error> {
    println!("Usage: {} path-to-json [path-to-json]

Arguments:
  path-to-json    Path to JSON file
", arg0);
    Ok(())
}


fn _main(files: &[String]) -> Result<(), std::io::Error> {
    for file in files {
        match parser::single_json(file) {
            Ok(is_valid) => println!("{} is {}valid JSON",
                file, if is_valid { "" } else { "not " }),
            Err(error) => println!("{} has error at ({}, {}): {}",
                file, error.row, error.col, error.msg)
        }
    }
    Ok(())
}


fn main() -> Result<(), std::io::Error> {
    let argv: Vec<String> = env::args().collect();
    let argc = argv.len();

    match argv.len() {
        1 => usage(&argv[0]),
        _ => _main(&argv[1..argc])
    }
}
