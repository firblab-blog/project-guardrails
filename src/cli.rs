use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::commands;
use crate::state::tasks::{TaskPriority, TaskStatus};

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
            Command::Brief(args) => commands::brief::run(args),
            Command::Resume(args) => commands::resume::run(args),
            Command::Timeline(args) => commands::timeline::run(args),
            Command::PreWork(args) => commands::pre_work::run(args),
            Command::Status(args) => commands::status::run(args),
            Command::Doctor(args) => commands::doctor::run(args),
            Command::Check(args) => commands::check::run(args),
            Command::Refresh(args) => commands::refresh::run(args),
            Command::Mcp(command) => match command {
                McpCommand::Serve(args) => commands::mcp::serve(args),
            },
            Command::PreCommit(args) => commands::pre_commit::run(args),
            Command::CommitMsgCheck(args) => commands::commit_msg_check::run(args),
            Command::Handoff(args) => commands::handoff::run(args),
            Command::Tasks(command) => commands::tasks::run(command),
            Command::Adapters(command) => match command {
                AdaptersCommand::List(args) => commands::adapters::list(args),
            },
            Command::Profiles(command) => match command {
                ProfilesCommand::List(args) => commands::profiles::list(args),
            },
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Install repo-local guardrails into a target repository.")]
    Init(InitArgs),
    Upgrade(UpgradeArgs),
    #[command(about = "Print a read-only paste-friendly LLM session starter.")]
    Brief(TargetArgs),
    #[command(about = "Print a read-only continuation view centered on the latest handoff.")]
    Resume(TargetArgs),
    #[command(about = "Print a read-only timeline of repo-local guardrails state.")]
    Timeline(TargetArgs),
    #[command(about = "Capture the structured repo context an agent should read before work.")]
    PreWork(PreWorkArgs),
    Status(StatusArgs),
    Doctor(TargetArgs),
    Check(TargetArgs),
    #[command(about = "Refresh profile-declared managed blocks without reapplying whole files.")]
    Refresh(RefreshArgs),
    #[command(subcommand)]
    Mcp(McpCommand),
    #[command(about = "Run staged-diff enforcement checks intended for pre-commit.")]
    PreCommit(TargetArgs),
    #[command(about = "Validate a commit message against repo-local task rules.")]
    CommitMsgCheck(CommitMsgCheckArgs),
    Handoff(HandoffArgs),
    #[command(subcommand)]
    Tasks(TasksCommand),
    #[command(subcommand)]
    Adapters(AdaptersCommand),
    #[command(subcommand)]
    Profiles(ProfilesCommand),
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
    long_about = "Install repo-local guardrails into a target repository.\n\nUse the default `minimal` profile for the neutral cross-language starting point. `docs-driven` keeps the neutral baseline and adds a required decision log. `guardrails` is the opt-in FirbLab-style doctrine profile with seeded operating guidance and curated best-practice docs. Use `--profile-path` when the profile lives outside the built-in set.",
    after_help = "Built-in profiles:\n  minimal      Neutral cross-language baseline with local config, AGENTS, tracker, handoff, and optional CI wiring. (default)\n  docs-driven  Neutral baseline plus a required decision log.\n  guardrails   Opt-in FirbLab-style doctrine profile with seeded docs/best-practices content.\n\nDiscover built-ins:\n  project-guardrails profiles list\n\nTypical next step:\n  project-guardrails init --target . --profile minimal --ci github"
)]
pub struct InitArgs {
    #[arg(long, default_value = ".", help = "Repository root to bootstrap.")]
    pub target: PathBuf,
    #[arg(
        long,
        default_value = "minimal",
        help = "Profile to install. `minimal` is the default neutral baseline."
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

#[derive(Debug, Subcommand, Clone)]
pub enum ProfilesCommand {
    #[command(about = "List built-in profiles and when to use them.")]
    List(ProfilesListArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ProfilesListArgs {
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Subcommand, Clone)]
pub enum AdaptersCommand {
    #[command(about = "List profile-declared adapter targets without installing files.")]
    List(TargetArgs),
}

#[derive(Debug, Args, Clone)]
pub struct PreWorkArgs {
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Args, Clone)]
pub struct StatusArgs {
    #[command(flatten)]
    pub target: TargetArgs,
    #[arg(long, help = "Emit the LLM-oriented machine-readable repo summary.")]
    pub for_llm: bool,
}

#[derive(Debug, Args, Clone)]
pub struct RefreshArgs {
    #[command(flatten)]
    pub target: TargetArgs,
    #[arg(long, help = "Report stale managed blocks without writing files.")]
    pub check: bool,
}

#[derive(Debug, Subcommand, Clone)]
pub enum McpCommand {
    #[command(about = "Serve repo-local guardrails tools over MCP stdio.")]
    Serve(McpServeArgs),
}

#[derive(Debug, Args, Clone)]
pub struct McpServeArgs {
    #[arg(
        long,
        default_value = ".",
        help = "Repository root exposed by this local MCP server."
    )]
    pub target: PathBuf,
}

#[derive(Debug, Args, Clone)]
pub struct HandoffArgs {
    #[command(subcommand)]
    pub command: Option<HandoffCommand>,
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Args, Clone)]
pub struct CommitMsgCheckArgs {
    #[arg(value_name = "MESSAGE_FILE")]
    pub message_file: PathBuf,
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Subcommand, Clone)]
pub enum HandoffCommand {
    #[command(about = "Print the current handoff template for compatibility.")]
    Print(TargetArgs),
    #[command(about = "List durable handoff records.")]
    List(TargetArgs),
    #[command(about = "Create a new handoff record under .guardrails/state/handoffs/.")]
    New(HandoffNewArgs),
    #[command(about = "Close an existing handoff record.")]
    Close(HandoffCloseArgs),
}

#[derive(Debug, Args, Clone)]
pub struct HandoffNewArgs {
    #[arg(long)]
    pub slug: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long = "task")]
    pub task_ids: Vec<u32>,
    #[arg(
        long,
        help = "Draft the handoff body from observable Git state with conservative caveats."
    )]
    pub from_git: bool,
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Args, Clone)]
pub struct HandoffCloseArgs {
    pub id: u32,
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Subcommand, Clone)]
pub enum TasksCommand {
    #[command(about = "List task records from .guardrails/state/tasks/.")]
    List(TasksListArgs),
    #[command(about = "Print one task record.")]
    Get(TasksGetArgs),
    #[command(about = "Create a new task record.")]
    New(TasksNewArgs),
    #[command(about = "Claim a task for an owner and move it into progress.")]
    Claim(TasksClaimArgs),
    #[command(about = "Update task metadata or status.")]
    Update(TasksUpdateArgs),
    #[command(about = "Close a task and record the commit that finished it.")]
    Close(TasksCloseArgs),
    #[command(about = "Validate every task record under .guardrails/state/tasks/.")]
    Lint(TasksLintArgs),
}

#[derive(Debug, Args, Clone)]
pub struct TasksListArgs {
    #[command(flatten)]
    pub target: TargetArgs,
    #[arg(long, value_enum)]
    pub status: Option<TaskStatus>,
    #[arg(long)]
    pub owner: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct TasksGetArgs {
    pub id: u32,
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Args, Clone)]
pub struct TasksNewArgs {
    #[arg(long)]
    pub slug: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long, value_enum)]
    pub priority: Option<TaskPriority>,
    #[arg(long)]
    pub owner: Option<String>,
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Args, Clone)]
pub struct TasksClaimArgs {
    pub id: u32,
    #[arg(long)]
    pub owner: String,
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Args, Clone)]
pub struct TasksUpdateArgs {
    pub id: u32,
    #[arg(long, value_enum)]
    pub status: Option<TaskStatus>,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long, value_enum)]
    pub priority: Option<TaskPriority>,
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Args, Clone)]
pub struct TasksCloseArgs {
    pub id: u32,
    #[arg(long)]
    pub commit: String,
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Args, Clone)]
pub struct TasksLintArgs {
    #[command(flatten)]
    pub target: TargetArgs,
}
