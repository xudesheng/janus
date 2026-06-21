use janus::otlp_ingest::{describe_scalar_resolution, format_text_summary, ingest_otlp_json_files};
use std::{env, path::PathBuf, process};

fn main() {
    match run() {
        Ok(exit_code) => process::exit(exit_code),
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn run() -> Result<i32, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut inputs = Vec::new();
    let mut json_summary = false;
    let mut source_ref = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--input" => {
                inputs.push(PathBuf::from(required_value(&args, index, "--input")?));
                index += 2;
            }
            "--json-summary" => {
                json_summary = true;
                index += 1;
            }
            "--ref" => {
                source_ref = Some(required_value(&args, index, "--ref")?);
                index += 2;
            }
            "-h" | "--help" => {
                print_usage();
                return Ok(0);
            }
            other => return Err(format!("unknown argument `{other}`").into()),
        }
    }

    if inputs.is_empty() {
        return Err("at least one --input path is required".into());
    }

    let result = ingest_otlp_json_files(&inputs)?;
    if json_summary {
        println!("{}", serde_json::to_string_pretty(&result.summary)?);
    } else {
        print!("{}", format_text_summary(&result.summary));
    }

    if let Some(source_ref) = source_ref {
        let description = describe_scalar_resolution(result.store.resolve_scalar_ref(&source_ref));
        println!("ref {source_ref}: {description}");
        if !description.starts_with("found ") {
            return Ok(1);
        }
    }

    Ok(i32::from(result.summary.has_errors()))
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn print_usage() {
    println!(
        "usage: ingest_otlp --input <path> [--input <path> ...] [--json-summary] [--ref <source-ref>]"
    );
}
