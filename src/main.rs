use std::{env, process};

use migratour::read_config_file;
fn main() {
    let args: Vec<String> = env::args().collect();

    let config = read_config_file().unwrap_or_else(|err|{
        eprintln!("error reading file {}",err);
        process::exit(1);
    });

    dbg!(config);
    
}
