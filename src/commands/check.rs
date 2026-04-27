use anyhow::{Result, bail};

use crate::{
    cli::{OutputFormat, TargetArgs},
    operations::check::run_check,
};

pub fn run(args: TargetArgs) -> Result<()> {
    let output = run_check(&args.target)?;

    if !output.ok {
        if matches!(args.format, OutputFormat::Json) {
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            for diagnostic in &output.diagnostics {
                eprintln!("[{}] {}", diagnostic.code, diagnostic.message);
            }
        }
        bail!("guardrails checks failed");
    }

    if matches!(args.format, OutputFormat::Json) {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("All configured local checks passed.");
    }
    Ok(())
}
