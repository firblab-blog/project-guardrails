use anyhow::Result;
use serde::Serialize;

use crate::{
    cli::{OutputFormat, ProfilesListArgs},
    output::JSON_SCHEMA_VERSION,
    profile::built_in_profile_infos,
};

pub fn list(args: ProfilesListArgs) -> Result<()> {
    let profiles = built_in_profile_infos()
        .into_iter()
        .map(|profile| ProfileListItem {
            name: profile.name.to_string(),
            summary: profile.summary.to_string(),
            description: profile.description.to_string(),
            is_default: profile.is_default,
            is_opt_in: profile.is_opt_in,
        })
        .collect::<Vec<_>>();

    if matches!(args.format, OutputFormat::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(&ProfilesListOutput {
                schema_version: JSON_SCHEMA_VERSION,
                profiles,
            })?
        );
        return Ok(());
    }

    println!("Built-in profiles");
    for profile in &profiles {
        let mut flags = Vec::new();
        if profile.is_default {
            flags.push("default");
        }
        if profile.is_opt_in {
            flags.push("opt-in");
        }

        if flags.is_empty() {
            println!("{}", profile.name);
        } else {
            println!("{} ({})", profile.name, flags.join(", "));
        }
        println!("  {}", profile.summary);
    }
    println!();
    println!("Use `project-guardrails init --profile <name>` to install one.");

    Ok(())
}

#[derive(Debug, Serialize)]
struct ProfilesListOutput {
    schema_version: u32,
    profiles: Vec<ProfileListItem>,
}

#[derive(Debug, Serialize)]
struct ProfileListItem {
    name: String,
    summary: String,
    description: String,
    is_default: bool,
    is_opt_in: bool,
}
