//! `devtidy`: see and safely reclaim disk space used by developer tooling.

mod clean_cmd;
mod format;
mod report;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use devtidy_core::{Ecosystem, Platform, Rule, ScanContext, scan};

/// Exit code for `scan --fail-above` when the threshold is exceeded.
const EXIT_ABOVE_THRESHOLD: u8 = 3;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Home directory to scan.
    #[arg(long, env = "DEVTIDY_HOME", global = true)]
    home: Option<PathBuf>,

    /// Directory with extra user rules (*.toml).
    #[arg(long, env = "DEVTIDY_RULES_DIR", global = true)]
    rules_dir: Option<PathBuf>,

    /// Print machine-readable JSON instead of a table.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show what developer tooling is using disk space.
    Scan {
        /// Only show these ecosystems.
        #[arg(long, short, value_enum)]
        ecosystem: Vec<EcosystemArg>,
        /// Include rules that matched nothing.
        #[arg(long)]
        all: bool,
        /// Exit with code 3 if the reclaimable total exceeds this size (e.g. 20G).
        #[arg(long, value_name = "SIZE", value_parser = format::parse_size)]
        fail_above: Option<u64>,
    },
    /// Explain what an item is and what removing it costs.
    Explain {
        /// Rule id, e.g. `gradle.caches`.
        id: String,
    },
    /// Reclaim space. Shows the plan and asks before changing anything.
    Clean {
        /// Rule ids to clean. A dangerous rule must be cleaned on its own.
        #[arg(required_unless_present = "safe", conflicts_with = "safe")]
        ids: Vec<String>,
        /// Clean every item marked safe.
        #[arg(long)]
        safe: bool,
        /// Show the plan without changing anything.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Do not ask for confirmation.
        #[arg(long, short)]
        yes: bool,
    },
    /// List every known rule.
    Rules,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum EcosystemArg {
    Ios,
    Android,
    Javascript,
    Flutter,
    Rust,
    Python,
    Docker,
    System,
}

impl From<EcosystemArg> for Ecosystem {
    fn from(arg: EcosystemArg) -> Self {
        match arg {
            EcosystemArg::Ios => Self::Ios,
            EcosystemArg::Android => Self::Android,
            EcosystemArg::Javascript => Self::Javascript,
            EcosystemArg::Flutter => Self::Flutter,
            EcosystemArg::Rust => Self::Rust,
            EcosystemArg::Python => Self::Python,
            EcosystemArg::Docker => Self::Docker,
            EcosystemArg::System => Self::System,
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let home = match cli.home {
        Some(home) => home,
        None => dirs::home_dir().context("could not determine the home directory")?,
    };
    let rules_dir = cli
        .rules_dir
        .unwrap_or_else(|| home.join(".config/devtidy/rules"));
    let rules = devtidy_rules::builtin_and_user(&rules_dir)?;
    let ctx = ScanContext {
        home,
        platform: Platform::current(),
    };

    match cli.command {
        Command::Scan {
            ecosystem,
            all,
            fail_above,
        } => {
            let ecosystems: Vec<Ecosystem> = ecosystem.into_iter().map(Into::into).collect();
            let selected: Vec<Rule> = rules
                .into_iter()
                .filter(|r| ecosystems.is_empty() || ecosystems.contains(&r.ecosystem))
                .collect();
            if std::io::stderr().is_terminal() && !cli.json {
                eprintln!("Scanning {} locations…", selected.len());
            }
            let findings = scan(&selected, &ctx);
            let report = report::ScanReport::new(&ctx, &selected, &findings);
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", format::scan_table(&report, all));
            }
            if fail_above.is_some_and(|limit| report.totals.reclaimable > limit) {
                return Ok(ExitCode::from(EXIT_ABOVE_THRESHOLD));
            }
        }
        Command::Explain { id } => {
            let Some(rule) = rules.iter().find(|r| r.id == id) else {
                bail!("unknown rule `{id}`; run `devtidy rules` to list them");
            };
            let finding = devtidy_core::scan::scan_rule(rule, &ctx);
            let item = report::Item::new(rule, &finding);
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report::Versioned::new(&item))?
                );
            } else {
                print!("{}", format::explain(rule, &item));
            }
        }
        Command::Clean {
            ids,
            safe,
            dry_run,
            yes,
        } => {
            let opts = clean_cmd::Options {
                ids,
                safe,
                dry_run,
                yes,
                json: cli.json,
            };
            return clean_cmd::run(&rules, &ctx, &opts, &history_log()?);
        }
        Command::Rules => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report::Versioned::new(report::RuleList {
                        rules: &rules
                    }))?
                );
            } else {
                print!("{}", format::rules_table(&rules));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// `history.jsonl` in the platform data directory, overridable for tests.
fn history_log() -> Result<PathBuf> {
    let dir = match std::env::var_os("DEVTIDY_DATA_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => dirs::data_dir()
            .context("could not determine the data directory")?
            .join("devtidy"),
    };
    Ok(dir.join("history.jsonl"))
}
