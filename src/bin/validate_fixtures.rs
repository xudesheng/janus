use janus::fixture_validation::{
    FixtureSelector, validate_fixture_corpus, validate_fixture_corpus_with_selector,
};
use std::{env, process};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut selector = FixtureSelector::default();
    let mut json = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                json = true;
                index += 1;
            }
            "--fixture" => {
                selector.fixture_id = Some(required_value(&args, index, "--fixture")?);
                index += 2;
            }
            "--capability" => {
                selector.capability = Some(required_value(&args, index, "--capability")?);
                index += 2;
            }
            "--failure-class" => {
                selector.failure_class = Some(required_value(&args, index, "--failure-class")?);
                index += 2;
            }
            "--difficulty" => {
                selector.difficulty = Some(required_value(&args, index, "--difficulty")?);
                index += 2;
            }
            "-h" | "--help" => {
                print_usage();
                return Ok(());
            }
            other => {
                return Err(format!("unknown argument `{other}`").into());
            }
        }
    }

    let report = if selector == FixtureSelector::default() {
        validate_fixture_corpus(".")
    } else {
        validate_fixture_corpus_with_selector(".", &selector)
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{report}");
    }

    if report.is_success() {
        Ok(())
    } else {
        process::exit(1);
    }
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn print_usage() {
    println!(
        "usage: validate_fixtures [--fixture <id>] [--capability <tag>] \
         [--failure-class <class>] [--difficulty <baseline|hard>] [--json]"
    );
}
