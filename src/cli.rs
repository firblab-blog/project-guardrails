use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::commands;

pub const CLI_DISPLAY_NAME: &str = "project-guardrails";

#[derive(Debug, Parser)]
#[command(
    name = CLI_DISPLAY_NAME,
    version,
    about = "Portable bootstrap utility for repo-local docs, rules, and guardrail flows."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        match self.command {
            Command::Init(args) => commands::init::run(args),
            Command::Upgrade(args) => commands::upgrade::run(args),
            Command::Status(args) => commands::status::run(args),
            Command::Doctor(args) => commands::doctor::run(args),
            Command::Check(args) => commands::check::run(args),
            Command::Handoff(args) => commands::handoff::run(args),
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Install repo-local guardrails into a target repository.")]
    Init(InitArgs),
    Upgrade(UpgradeArgs),
    Status(TargetArgs),
    Doctor(TargetArgs),
    Check(TargetArgs),
    Handoff(TargetArgs),
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CiProvider {
    Github,
    Gitlab,
    None,
}

impl CiProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::None => "none",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "github" => Some(Self::Github),
            "gitlab" => Some(Self::Gitlab),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Args, Clone)]
pub struct TargetArgs {
    #[arg(long, default_value = ".")]
    pub target: PathBuf,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Args, Clone)]
#[command(
    long_about = "Install repo-local guardrails into a target repository.\n\nUse the default `minimal` profile for the smallest cross-language starting point. `docs-driven` adds a required decision log for teams that want stronger documentation discipline. Use `--profile-path` when the profile lives outside the built-in set.",
    after_help = "Built-in profiles:\n  minimal      Smallest cross-language starting point. Installs local config, AGENTS, tracker, handoff, and optional CI wiring. (default)\n  docs-driven  Everything in minimal, plus a required decision log.\n\nTypical next step:\n  project-guardrails init --target . --profile minimal --ci github"
)]
pub struct InitArgs {
    #[arg(long, default_value = ".", help = "Repository root to bootstrap.")]
    pub target: PathBuf,
    #[arg(
        long,
        default_value = "minimal",
        help = "Profile to install. `minimal` is the default happy path."
    )]
    pub profile: String,
    #[arg(
        long,
        help = "Directory containing a custom profile when it is not built in."
    )]
    pub profile_path: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        help = "CI provider to wire in. If omitted, the profile default is used."
    )]
    pub ci: Option<CiProvider>,
    #[arg(long, help = "Preview the install plan without writing files.")]
    pub dry_run: bool,
    #[arg(long, help = "Overwrite files that guardrails manages.")]
    pub force: bool,
}

#[derive(Debug, Args, Clone)]
pub struct UpgradeArgs {
    #[arg(long, default_value = ".")]
    pub target: PathBuf,
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long)]
    pub profile_path: Option<PathBuf>,
    #[arg(long)]
    pub ci: Option<CiProvider>,
    #[arg(long)]
    pub plan: bool,
    #[arg(long)]
    pub apply: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}
