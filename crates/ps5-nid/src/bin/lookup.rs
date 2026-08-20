use std::env;
use ps5_nid::hash;

fn print_usage(prog: &str) {
    eprintln!("Usage: {} <function_name>", prog);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }
    let name = &args[1];
    let nid = hash(name);
    println!("{}", nid);
}
