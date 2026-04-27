use anyhow::{Result, bail};

use crate::{
    cli::{OutputFormat, RefreshArgs},
    operations::refresh::{RefreshOutput, refresh},
};

pub fn run(args: RefreshArgs) -> Result<()> {
    let report = refresh(&args.target.target, args.check)?;

    if matches!(args.target.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_refresh_report(&report);
    }

    if !report.ok {
        if !matches!(args.target.format, OutputFormat::Json) {
            for diagnostic in &report.diagnostics {
                eprintln!("[{}] {}", diagnostic.code, diagnostic.message);
            }
        }

        if args.check && report.changed {
            bail!("managed blocks are stale");
        }
        bail!("managed block refresh failed");
    }

    Ok(())
}

fn print_refresh_report(report: &RefreshOutput) {
    println!("Guardrails refresh");
    println!("repo_root={}", report.repo_root);
    println!("check={}", report.check);
    println!("changed={}", report.changed);

    if report.changed_paths.is_empty() {
        println!("changed_paths=none");
    } else {
        println!("changed_paths:");
        for path in &report.changed_paths {
            println!("  - {}", path);
        }
    }

    if report.blocks.is_empty() {
        println!("blocks=none");
    } else {
        println!("blocks:");
        for block in &report.blocks {
            println!(
                "  - {} id={} generator={} status={}",
                block.path,
                block.id,
                block.generator,
                block.status.as_str()
            );
        }
    }
}
