use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{RUNS_DIR, runs_dir};

pub fn generate_run_id(repo_root: &Path) -> String {
    let now = Utc::now();
    let mut hasher = Sha256::new();
    hasher.update(repo_root.display().to_string().as_bytes());
    hasher.update(now.to_rfc3339().as_bytes());
    hasher.update(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string()
            .as_bytes(),
    );
    let digest = format!("{:x}", hasher.finalize());
    format!("{}-{}", now.format("%Y%m%dT%H%M%SZ"), &digest[..8])
}

pub fn run_relative_path(kind: &str, run_id: &str) -> String {
    format!("{RUNS_DIR}/{kind}-{run_id}.json")
}

pub fn write_json_run<T>(repo_root: &Path, kind: &str, run_id: &str, value: &T) -> Result<String>
where
    T: Serialize,
{
    fs::create_dir_all(runs_dir(repo_root))
        .with_context(|| format!("failed to create {}", runs_dir(repo_root).display()))?;

    let relative_path = run_relative_path(kind, run_id);
    let destination = repo_root.join(&relative_path);
    fs::write(&destination, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("failed to write {}", destination.display()))?;

    Ok(relative_path)
}
