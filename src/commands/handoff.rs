use std::fs;

use anyhow::{Context, Result};

use crate::{cli::TargetArgs, config::detect_repo_root};

pub fn run(args: TargetArgs) -> Result<()> {
    let repo_root = detect_repo_root(&args.target)?;
    let template_path = repo_root.join("docs/project/handoff-template.md");
    let contents = fs::read_to_string(&template_path)
        .with_context(|| format!("failed to read {}", template_path.display()))?;
    println!("{contents}");
    Ok(())
}
