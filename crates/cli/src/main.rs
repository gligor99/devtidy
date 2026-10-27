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
        /// Do not save this scan to the history database.
        #[arg(long)]
        no_record: bool,
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
    /// Show past cleanups and how much they freed.
    History {
        /// Number of cleanups to show.
        #[arg(long, default_value_t = 20)]
        limit: u32,
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
            no_record,
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
            if !no_record && let Err(err) = record_scan(&ctx, &findings) {
                eprintln!("warning: could not record scan: {err:#}");
            }
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
            return clean_cmd::run(&rules, &ctx, &opts);
        }
        Command::History { limit } => {
            let records = devtidy_store::Store::open_default()?.cleanups(limit)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report::Versioned::new(report::History {
                        cleanups: &records
                    }))?
                );
            } else {
                print!("{}", format::history(&records));
            }
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

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

fn record_scan(ctx: &ScanContext, findings: &[devtidy_core::Finding]) -> Result<()> {
    let mut store = devtidy_store::Store::open_default()?;
    store.record_scan(now(), devtidy_core::disk::space(&ctx.home), findings)?;
    Ok(())
}

fn record_cleanup(report: &devtidy_core::Report) -> Result<()> {
    devtidy_store::Store::open_default()?.record_cleanup(now(), report)?;
    Ok(())
}
