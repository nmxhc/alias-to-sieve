use alias_to_sieve::*;
use fqdn::FQDN;
use std::env;
use std::str::FromStr;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() % 2 == 0 {
        print_help();
        return;
    }

    // Collect alias files and their default domains
    let mut alias_files: Vec<AliasFile> = Vec::new();
    for i in (1..args.len()).step_by(2) {
        if let Ok(lines) = read_lines(&args[i]) {
            alias_files.push(AliasFile {
                content: lines,
                default_domain: FQDN::from_str(&args[i + 1]).unwrap(),
            });
        }
    }
    println!(
        "{}",
        generate_sieve_script(parse_alias_to_map(alias_files).unwrap())
    );
}

fn print_help() {
    print!(
        "Reads a virtual alias file and needs a default domain to append to local paths, e.g.
    ./alias_to_sieve example.com.txt example.com"
    );
}
