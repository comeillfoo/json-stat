use std::env;

use lib::stat;


fn usage(arg0: &String) -> Result<(), std::io::Error> {
    println!("Usage: {} path-to-json [path-to-json]

Arguments:
  path-to-json    Path to JSON file
", arg0);
    Ok(())
}


fn wrapper(files: &[String]) -> Result<(), std::io::Error> {
    let result = stat::multiple_jsons(files);
    Ok(())
}


fn main() -> Result<(), std::io::Error> {
    let argv: Vec<String> = env::args().collect();
    let argc = argv.len();

    match argv.len() {
        1 => usage(&argv[0]),
        _ => wrapper(&argv[1..argc])
    }
}
