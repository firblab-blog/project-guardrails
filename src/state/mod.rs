use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Serialize, de::DeserializeOwned};

pub mod handoffs;
pub mod runs;
pub mod tasks;

pub const STATE_DIR: &str = ".guardrails/state";
pub const TASKS_DIR: &str = ".guardrails/state/tasks";
pub const HANDOFFS_DIR: &str = ".guardrails/state/handoffs";
pub const RUNS_DIR: &str = ".guardrails/state/runs";

pub fn tasks_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(TASKS_DIR)
}

pub fn handoffs_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(HANDOFFS_DIR)
}

pub fn runs_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(RUNS_DIR)
}

pub fn ensure_state_layout(repo_root: &Path) -> Result<()> {
    fs::create_dir_all(tasks_dir(repo_root))
        .with_context(|| format!("failed to create {}", tasks_dir(repo_root).display()))?;
    fs::create_dir_all(handoffs_dir(repo_root))
        .with_context(|| format!("failed to create {}", handoffs_dir(repo_root).display()))?;
    fs::create_dir_all(runs_dir(repo_root))
        .with_context(|| format!("failed to create {}", runs_dir(repo_root).display()))?;
    Ok(())
}

pub fn now_timestamp() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn title_from_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn is_kebab_case(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
}

pub fn parse_toml_frontmatter<T>(raw: &str) -> Result<(T, String)>
where
    T: DeserializeOwned,
{
    let normalized = raw.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    let Some(delimiter) = lines.next() else {
        bail!("missing frontmatter delimiter")
    };

    if delimiter.trim() != "+++" && delimiter.trim() != "---" {
        bail!("expected TOML frontmatter delimited by `+++` or `---`")
    }

    let mut frontmatter = Vec::new();
    let mut found_closing = false;
    let delimiter = delimiter.trim().to_string();

    for line in lines.by_ref() {
        if line.trim() == delimiter {
            found_closing = true;
            break;
        }
        frontmatter.push(line);
    }

    if !found_closing {
        bail!("frontmatter block is missing a closing delimiter")
    }

    let body = lines.collect::<Vec<_>>().join("\n");
    let parsed =
        toml::from_str(&frontmatter.join("\n")).context("failed to parse TOML frontmatter")?;
    Ok((parsed, body.trim_start_matches('\n').to_string()))
}

pub fn render_toml_frontmatter<T>(frontmatter: &T, body: &str) -> Result<String>
where
    T: Serialize,
{
    let frontmatter =
        toml::to_string_pretty(frontmatter).context("failed to serialize frontmatter")?;
    Ok(format!(
        "+++\n{}+++\n\n{}",
        frontmatter,
        body.trim_start_matches('\n')
    ))
}
