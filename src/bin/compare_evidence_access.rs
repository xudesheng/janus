use janus::comparative_eval::{
    EvalBudget, EvalFixtureSelector, format_text_report, load_comparative_eval_report,
    regression_gate_failure_message,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Json,
    Text,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut selector = EvalFixtureSelector::default();
    let mut budget = EvalBudget::default();
    let mut format = OutputFormat::Text;
    let mut output = PathBuf::from("target/eval/comparative-eval-v1.json");
    let mut all = false;
    let mut fail_on_regression = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--all" => {
                all = true;
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
            "--trap" => {
                selector.false_causality_trap =
                    Some(parse_bool(&required_value(&args, index, "--trap")?)?);
                index += 2;
            }
            "--max-items" => {
                budget.max_items =
                    parse_positive_u32(&required_value(&args, index, "--max-items")?)?;
                index += 2;
            }
            "--max-tokens" => {
                budget.max_tokens =
                    parse_positive_u32(&required_value(&args, index, "--max-tokens")?)?;
                index += 2;
            }
            "--format" => {
                format = parse_format(&required_value(&args, index, "--format")?)?;
                index += 2;
            }
            "--output" => {
                output = PathBuf::from(required_value(&args, index, "--output")?);
                index += 2;
            }
            "--fail-on-regression" => {
                fail_on_regression = true;
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

    if all && has_selector(&selector) {
        return Err("pass --all or selector flags, not both".into());
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_sha = current_repo_sha(root);
    let report = load_comparative_eval_report(root, &selector, budget, repo_sha)?;
    let json = serde_json::to_string_pretty(&report)?;

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, json.as_bytes())?;

    match format {
        OutputFormat::Json => println!("{json}"),
        OutputFormat::Text => print!("{}", format_text_report(&report)),
    }

    if fail_on_regression && let Some(message) = regression_gate_failure_message(&report) {
        return Err(message.into());
    }

    Ok(())
}

fn has_selector(selector: &EvalFixtureSelector) -> bool {
    selector.fixture_id.is_some()
        || selector.capability.is_some()
        || selector.failure_class.is_some()
        || selector.difficulty.is_some()
        || selector.false_causality_trap.is_some()
}

fn current_repo_sha(root: &Path) -> String {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(root)
        .output();

    output
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|sha| sha.trim().to_string())
        .filter(|sha| !sha.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn required_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_positive_u32(value: &str) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("expected positive integer, got `{value}`"))?;
    if parsed == 0 {
        Err("value must be greater than zero".to_string())
    } else {
        Ok(parsed)
    }
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("expected true or false, got `{value}`")),
    }
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "json" => Ok(OutputFormat::Json),
        "text" => Ok(OutputFormat::Text),
        _ => Err(format!("expected json or text, got `{value}`")),
    }
}

fn print_usage() {
    println!(
        "usage: compare_evidence_access [--all | --fixture <id>] \
         [--capability <tag>] [--failure-class <class>] \
         [--difficulty <baseline|hard>] [--trap <true|false>] \
         [--max-items <n>] [--max-tokens <n>] [--format json|text] \
         [--output <path>] [--fail-on-regression]"
    );
}
