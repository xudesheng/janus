use janus::{
    fixture_simulator::{
        format_dry_run_plan, format_jsonl_plan, format_replay_summary, plan_fixture_replay,
        replay_fixture_case,
    },
    fixture_validation::{FixtureCorpus, FixtureSelector},
};
use std::{env, path::Path, process};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut fixture_id = None;
    let mut dry_run = false;
    let mut jsonl = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--fixture" => {
                fixture_id = Some(required_value(&args, index, "--fixture")?);
                index += 2;
            }
            "--dry-run" => {
                dry_run = true;
                index += 1;
            }
            "--jsonl" => {
                jsonl = true;
                index += 1;
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

    if dry_run && jsonl {
        return Err("pass either --dry-run or --jsonl, not both".into());
    }

    let fixture_id = fixture_id.ok_or("--fixture requires a value")?;
    let corpus = FixtureCorpus::load(Path::new(env!("CARGO_MANIFEST_DIR")))?;
    let selector = FixtureSelector {
        fixture_id: Some(fixture_id.clone()),
        ..FixtureSelector::default()
    };
    let case = corpus
        .select(&selector)
        .into_iter()
        .next()
        .ok_or_else(|| format!("fixture `{fixture_id}` not found"))?;
    if dry_run || jsonl {
        let plan = plan_fixture_replay(case)?;
        if jsonl {
            println!("{}", format_jsonl_plan(&plan)?);
        } else {
            print!("{}", format_dry_run_plan(&plan));
        }
    } else {
        let summary = replay_fixture_case(case)?;
        print!("{}", format_replay_summary(&summary));
    }

    Ok(())
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn print_usage() {
    println!("usage: simulate_fixture --fixture <id> [--dry-run | --jsonl]");
}
