// Re-export the lib so the binary can use the same modules.
use eval_runner::{report, runner, validate};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "eval-runner", about = "NeoMind chat agent eval system runner")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a single case and emit CaseRecord JSON on stdout.
    RunCase { case_file: String },
    /// Validate case schema without running.
    ValidateCase { case_file: String },
    /// Aggregate scores.jsonl → grade-card.md.
    Report {
        scores_jsonl: String,
        #[arg(long, default_value = "grade-card.md")]
        out: String,
    },
    /// Run the smoke suite (good-001..003, bad-001..002).
    Smoke {
        #[arg(long, default_value = "eval/smoke")]
        dir: String,
    },
    /// Validate all cases under a directory tree.
    ValidateAll { root: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::RunCase { case_file } => {
            let record = runner::run_case(&case_file).await?;
            println!("{}", serde_json::to_string(&record)?);
        }
        Cmd::ValidateCase { case_file } => {
            let raw = std::fs::read_to_string(&case_file)?;
            let v: serde_json::Value = serde_json::from_str(&raw)?;
            let report = validate::validate_case(&v)?;
            if report.errors.is_empty() {
                println!("OK");
            } else {
                for e in &report.errors {
                    eprintln!("ERROR: {}", e);
                }
                std::process::exit(1);
            }
        }
        Cmd::ValidateAll { root } => {
            let mut total = 0;
            let mut failed = 0;
            for entry in walkdir(&root) {
                if entry.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                total += 1;
                let raw = match std::fs::read_to_string(&entry) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("{}: read error: {}", entry.display(), e);
                        failed += 1;
                        continue;
                    }
                };
                let v: serde_json::Value = match serde_json::from_str(&raw) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("{}: parse error: {}", entry.display(), e);
                        failed += 1;
                        continue;
                    }
                };
                let report = validate::validate_case(&v)?;
                if !report.errors.is_empty() {
                    failed += 1;
                    eprintln!("{}:", entry.display());
                    for e in &report.errors {
                        eprintln!("  ERROR: {}", e);
                    }
                }
            }
            println!("validated {} cases, {} failed", total, failed);
            if failed > 0 {
                std::process::exit(1);
            }
        }
        Cmd::Report { scores_jsonl, out } => {
            let raw = std::fs::read_to_string(&scores_jsonl)?;
            let agg = report::aggregate(&raw)?;
            let out_path = std::path::Path::new(&out);
            report::write_grade_card(&agg, out_path)?;
            println!(
                "grade: {} ({:.1}/100), {} cases, {} malformed, {} agent errors",
                report::grade_letter(agg.overall()),
                agg.overall(),
                agg.total_cases,
                agg.malformed,
                agg.agent_errors
            );
            println!("wrote {}", out_path.display());
        }
        Cmd::Smoke { dir } => {
            // Runs every *.json in `dir` sequentially and prints CaseRecords to stdout.
            for entry in walkdir(&dir) {
                if entry.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                eprintln!("--- {} ---", entry.display());
                match runner::run_case(&entry.to_string_lossy()).await {
                    Ok(record) => println!("{}", serde_json::to_string(&record)?),
                    Err(e) => eprintln!("FATAL: {}", e),
                }
            }
        }
    }
    Ok(())
}

/// Recursively collect files under a directory.
fn walkdir(root: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(root)];
    while let Some(p) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&p) else {
            continue;
        };
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}
