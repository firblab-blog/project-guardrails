use std::path::Path;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    config::GuardrailsConfig,
    profile::ManagedBlockConfig,
    state::{
        handoffs::{self, HandoffRecord, HandoffStatus},
        tasks::{self, TaskRecord, TaskStatus},
    },
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ManagedBlockPlacement {
    Prepend,
    #[default]
    AfterFirstHeading,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedBlock {
    pub id: String,
    pub generator: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedManagedBlock {
    pub id: String,
    pub generator: Option<String>,
    pub content: String,
}

const START_PREFIX: &str = "<!-- guardrails:managed start ";
const END_PREFIX: &str = "<!-- guardrails:managed end ";

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

pub fn sha256_text(contents: &str) -> String {
    sha256_bytes(contents.as_bytes())
}

pub fn parse_managed_blocks(raw: &str) -> Result<Vec<ParsedManagedBlock>> {
    let normalized = normalize_newlines(raw);
    let lines = normalized.lines().collect::<Vec<_>>();
    let mut blocks = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index].trim();
        if !line.starts_with(START_PREFIX) {
            index += 1;
            continue;
        }

        let start = parse_start_marker(line)?;
        let mut content = Vec::new();
        index += 1;

        let mut found_end = false;
        while index < lines.len() {
            let current = lines[index].trim();
            if current.starts_with(END_PREFIX) {
                let end_id = parse_end_marker(current)?;
                if end_id != start.id {
                    bail!(
                        "managed block end marker id `{end_id}` does not match start id `{}`",
                        start.id
                    );
                }
                found_end = true;
                break;
            }
            content.push(lines[index]);
            index += 1;
        }

        if !found_end {
            bail!("managed block `{}` is missing an end marker", start.id);
        }

        blocks.push(ParsedManagedBlock {
            id: start.id,
            generator: start.generator,
            content: content.join("\n"),
        });
        index += 1;
    }

    Ok(blocks)
}

pub fn upsert_managed_block(
    raw: &str,
    block: &ManagedBlock,
    placement: ManagedBlockPlacement,
) -> Result<String> {
    let normalized = normalize_newlines(raw);
    let rendered = render_managed_block(block);
    let lines = normalized.lines().collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut index = 0;
    let mut replaced = false;

    while index < lines.len() {
        let line = lines[index].trim();
        if !line.starts_with(START_PREFIX) {
            output.push(lines[index].to_string());
            index += 1;
            continue;
        }

        let start = parse_start_marker(line)?;
        if start.id != block.id {
            output.push(lines[index].to_string());
            index += 1;
            index = copy_managed_block_through_end(&lines, &start.id, index, &mut output)?;
            continue;
        }

        output.push(rendered.clone());
        replaced = true;
        index += 1;
        index = skip_managed_block_through_end(&lines, &block.id, index)?;
    }

    let document = if replaced {
        output.join("\n")
    } else {
        insert_rendered_block(&normalized, &rendered, placement)
    };

    Ok(ensure_trailing_newline(&document))
}

fn copy_managed_block_through_end(
    lines: &[&str],
    expected_id: &str,
    mut index: usize,
    output: &mut Vec<String>,
) -> Result<usize> {
    while index < lines.len() {
        let current = lines[index].trim();
        output.push(lines[index].to_string());
        index += 1;

        if !current.starts_with(END_PREFIX) {
            continue;
        }

        let end_id = parse_end_marker(current)?;
        if end_id != expected_id {
            bail!("managed block end marker id `{end_id}` does not match start id `{expected_id}`");
        }
        break;
    }

    Ok(index)
}

fn skip_managed_block_through_end(
    lines: &[&str],
    expected_id: &str,
    mut index: usize,
) -> Result<usize> {
    while index < lines.len() {
        let current = lines[index].trim();
        index += 1;

        if !current.starts_with(END_PREFIX) {
            continue;
        }

        let end_id = parse_end_marker(current)?;
        if end_id != expected_id {
            bail!("managed block end marker id `{end_id}` does not match start id `{expected_id}`");
        }
        return Ok(index);
    }

    bail!("managed block `{expected_id}` is missing an end marker")
}

pub fn render_managed_block(block: &ManagedBlock) -> String {
    let mut start = format!(
        "<!-- guardrails:managed start id={} generator={} -->",
        block.id, block.generator
    );
    if block.generator.is_empty() {
        start = format!("<!-- guardrails:managed start id={} -->", block.id);
    }

    format!(
        "{start}\n{}\n<!-- guardrails:managed end id={} -->",
        block.content.trim_end(),
        block.id
    )
}

pub fn render_declared_block(
    repo_root: &Path,
    config: &GuardrailsConfig,
    spec: &ManagedBlockConfig,
) -> Result<ManagedBlock> {
    let content = match spec.generator.as_str() {
        "repo_context_v1" => render_repo_context_block(repo_root, config),
        "tracker_sync_v1" => render_tracker_sync_block(repo_root),
        other => bail!("unknown managed block generator `{other}`"),
    };

    Ok(ManagedBlock {
        id: spec.id.clone(),
        generator: spec.generator.clone(),
        content,
    })
}

fn insert_rendered_block(raw: &str, rendered: &str, placement: ManagedBlockPlacement) -> String {
    match placement {
        ManagedBlockPlacement::Prepend => {
            let body = raw.trim_start_matches('\n');
            if body.is_empty() {
                rendered.to_string()
            } else {
                format!("{rendered}\n\n{body}")
            }
        }
        ManagedBlockPlacement::AfterFirstHeading => insert_after_first_heading(raw, rendered),
    }
}

fn insert_after_first_heading(raw: &str, rendered: &str) -> String {
    let lines = raw.lines().collect::<Vec<_>>();
    let heading_index = lines
        .iter()
        .position(|line| line.trim_start().starts_with("# "));

    let Some(heading_index) = heading_index else {
        let body = raw.trim_start_matches('\n');
        if body.is_empty() {
            return rendered.to_string();
        }
        return format!("{rendered}\n\n{body}");
    };

    let before = lines[..=heading_index].join("\n");
    let after = lines[heading_index + 1..].join("\n");
    let trimmed_after = after.trim_start_matches('\n');

    if trimmed_after.is_empty() {
        format!("{before}\n\n{rendered}")
    } else {
        format!("{before}\n\n{rendered}\n\n{trimmed_after}")
    }
}

fn ensure_trailing_newline(value: &str) -> String {
    if value.ends_with('\n') {
        value.to_string()
    } else {
        format!("{value}\n")
    }
}

fn normalize_newlines(raw: &str) -> String {
    raw.replace("\r\n", "\n")
}

fn parse_start_marker(line: &str) -> Result<StartMarker> {
    let Some(inner) = line
        .strip_prefix(START_PREFIX)
        .and_then(|value| value.strip_suffix("-->"))
    else {
        bail!("invalid managed block start marker: {line}");
    };

    let mut id = None;
    let mut generator = None;

    for token in inner.split_whitespace() {
        if let Some(value) = token.strip_prefix("id=") {
            id = Some(value.to_string());
        } else if let Some(value) = token.strip_prefix("generator=") {
            generator = Some(value.to_string());
        }
    }

    let Some(id) = id else {
        bail!("managed block start marker is missing an id: {line}");
    };

    Ok(StartMarker { id, generator })
}

fn parse_end_marker(line: &str) -> Result<String> {
    let Some(inner) = line
        .strip_prefix(END_PREFIX)
        .and_then(|value| value.strip_suffix("-->"))
    else {
        bail!("invalid managed block end marker: {line}");
    };

    for token in inner.split_whitespace() {
        if let Some(value) = token.strip_prefix("id=") {
            return Ok(value.to_string());
        }
    }

    bail!("managed block end marker is missing an id: {line}")
}

struct StartMarker {
    id: String,
    generator: Option<String>,
}

fn render_repo_context_block(repo_root: &Path, config: &GuardrailsConfig) -> String {
    let active_tasks = active_tasks(repo_root);
    let open_handoffs = open_handoffs(repo_root);
    let mut lines = vec![
        "## Managed Repo Context".to_string(),
        String::new(),
        "This block is tool-managed. It strengthens proxy enforcement and freshness signals, but it does not prove that a human or LLM read or understood the repository.".to_string(),
        String::new(),
        "### Required Context Paths".to_string(),
    ];

    for required in required_context_paths(config) {
        lines.push(format!("- `{required}`"));
    }

    lines.push(String::new());
    lines.push("### Active Tasks".to_string());
    if active_tasks.is_empty() {
        lines.push(
            "- no active repo-local tasks are recorded under `.guardrails/state/tasks/`"
                .to_string(),
        );
    } else {
        for task in active_tasks.iter().take(3) {
            lines.push(format!("- {}", summarize_task(task)));
        }
    }

    lines.push(String::new());
    lines.push("### Open Handoffs".to_string());
    if open_handoffs.is_empty() {
        lines.push(
            "- no open handoffs are recorded under `.guardrails/state/handoffs/`".to_string(),
        );
    } else {
        for handoff in open_handoffs.iter().take(2) {
            lines.push(format!("- {}", summarize_handoff(handoff)));
        }
    }

    lines.join("\n")
}

fn render_tracker_sync_block(repo_root: &Path) -> String {
    let active_tasks = active_tasks(repo_root);
    let open_handoffs = open_handoffs(repo_root);
    let recent_handoff = all_handoffs(repo_root)
        .into_iter()
        .max_by(|left, right| left.frontmatter.updated.cmp(&right.frontmatter.updated));
    let mut lines = vec![
        "## Managed Task Snapshot".to_string(),
        String::new(),
        "This block is tool-managed. It keeps repo-local task and handoff state visible in the tracker, but it does not prove that a contributor followed the documented workflow.".to_string(),
        String::new(),
        "### Active Tasks".to_string(),
    ];

    if active_tasks.is_empty() {
        lines.push("- no active tasks are currently approved or in progress".to_string());
    } else {
        for task in active_tasks {
            lines.push(format!("- {}", summarize_task(&task)));
        }
    }

    lines.push(String::new());
    lines.push("### Handoff Status".to_string());
    if let Some(handoff) = open_handoffs.first() {
        lines.push(format!("- open: {}", summarize_handoff(handoff)));
    } else if let Some(handoff) = recent_handoff {
        lines.push(format!(
            "- most recent: {} (closed)",
            summarize_handoff(&handoff)
        ));
    } else {
        lines.push("- no handoffs have been recorded yet".to_string());
    }

    lines.join("\n")
}

fn required_context_paths(config: &GuardrailsConfig) -> Vec<String> {
    let mut paths = config.docs.required.clone();
    if !paths.iter().any(|path| path == "AGENTS.md") {
        paths.insert(0, "AGENTS.md".to_string());
    }
    paths
}

fn active_tasks(repo_root: &Path) -> Vec<TaskRecord> {
    let mut tasks = tasks::load_collection(repo_root)
        .map(|collection| collection.tasks)
        .unwrap_or_default()
        .into_iter()
        .filter(|task| {
            matches!(
                task.frontmatter.status,
                TaskStatus::Approved | TaskStatus::InProgress | TaskStatus::Blocked
            )
        })
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| right.frontmatter.updated.cmp(&left.frontmatter.updated));
    tasks
}

fn open_handoffs(repo_root: &Path) -> Vec<HandoffRecord> {
    let mut handoffs = all_handoffs(repo_root)
        .into_iter()
        .filter(|handoff| handoff.frontmatter.status == HandoffStatus::Open)
        .collect::<Vec<_>>();
    handoffs.sort_by(|left, right| right.frontmatter.updated.cmp(&left.frontmatter.updated));
    handoffs
}

fn all_handoffs(repo_root: &Path) -> Vec<HandoffRecord> {
    handoffs::load_all(repo_root).unwrap_or_default()
}

fn summarize_task(task: &TaskRecord) -> String {
    let owner = task
        .frontmatter
        .owner
        .as_deref()
        .map(|value| format!(", owner {value}"))
        .unwrap_or_default();
    format!(
        "`{}` {} ({}{owner}, updated {})",
        task.id_string(),
        task.frontmatter.title,
        task.frontmatter.status.as_str(),
        task.frontmatter.updated
    )
}

fn summarize_handoff(handoff: &HandoffRecord) -> String {
    format!(
        "`{}` {} ({} task link(s), updated {})",
        handoff.id_string(),
        handoff.frontmatter.title,
        handoff.frontmatter.task_ids.len(),
        handoff.frontmatter.updated
    )
}

#[cfg(test)]
mod tests {
    use super::{
        ManagedBlock, ManagedBlockPlacement, parse_managed_blocks, render_managed_block,
        sha256_text, upsert_managed_block,
    };

    #[test]
    fn parse_managed_blocks_reads_block_metadata_and_content() {
        let document = "\
# AGENTS.md

<!-- guardrails:managed start id=repo-context generator=repo_context_v1 -->
## Managed Context

- one
<!-- guardrails:managed end id=repo-context -->
";

        let blocks = parse_managed_blocks(document).expect("blocks");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id, "repo-context");
        assert_eq!(blocks[0].generator.as_deref(), Some("repo_context_v1"));
        assert!(blocks[0].content.contains("## Managed Context"));
    }

    #[test]
    fn upsert_managed_block_replaces_existing_block_idempotently() {
        let existing = "\
# AGENTS.md

<!-- guardrails:managed start id=repo-context generator=repo_context_v1 -->
old
<!-- guardrails:managed end id=repo-context -->

## Human Section
";
        let block = ManagedBlock {
            id: "repo-context".to_string(),
            generator: "repo_context_v1".to_string(),
            content: "new".to_string(),
        };

        let updated =
            upsert_managed_block(existing, &block, ManagedBlockPlacement::AfterFirstHeading)
                .expect("updated");
        let updated_again =
            upsert_managed_block(&updated, &block, ManagedBlockPlacement::AfterFirstHeading)
                .expect("updated again");

        assert_eq!(updated, updated_again);
        assert!(updated.contains("new"));
        assert!(updated.contains("## Human Section"));
    }

    #[test]
    fn upsert_managed_block_inserts_after_heading_when_missing() {
        let existing = "# Tracker\n\n## Human Notes\n";
        let block = ManagedBlock {
            id: "task-sync".to_string(),
            generator: "tracker_sync_v1".to_string(),
            content: "## Managed Task Snapshot".to_string(),
        };

        let updated =
            upsert_managed_block(existing, &block, ManagedBlockPlacement::AfterFirstHeading)
                .expect("updated");

        let expected_prefix = format!(
            "# Tracker\n\n{}\n\n## Human Notes",
            render_managed_block(&block)
        );
        assert!(updated.starts_with(&expected_prefix));
    }

    #[test]
    fn parse_managed_blocks_rejects_mismatched_end_ids() {
        let document = "\
<!-- guardrails:managed start id=repo-context generator=repo_context_v1 -->
body
<!-- guardrails:managed end id=tracker-context -->
";

        let error = parse_managed_blocks(document).expect_err("mismatch should fail");
        assert!(error.to_string().contains("does not match start id"));
    }

    #[test]
    fn upsert_managed_block_preserves_unrelated_managed_blocks() {
        let existing = "\
# AGENTS.md

<!-- guardrails:managed start id=other generator=repo_context_v1 -->
keep me
<!-- guardrails:managed end id=other -->
";
        let block = ManagedBlock {
            id: "repo-context".to_string(),
            generator: "repo_context_v1".to_string(),
            content: "new".to_string(),
        };

        let updated =
            upsert_managed_block(existing, &block, ManagedBlockPlacement::AfterFirstHeading)
                .expect("updated");

        assert!(updated.contains("keep me"));
        assert!(updated.contains("<!-- guardrails:managed end id=other -->"));
        assert!(updated.contains("<!-- guardrails:managed start id=repo-context"));
    }

    #[test]
    fn upsert_managed_block_rejects_missing_end_marker_for_target_block() {
        let existing = "\
<!-- guardrails:managed start id=repo-context generator=repo_context_v1 -->
old
";
        let block = ManagedBlock {
            id: "repo-context".to_string(),
            generator: "repo_context_v1".to_string(),
            content: "new".to_string(),
        };

        let error =
            upsert_managed_block(existing, &block, ManagedBlockPlacement::AfterFirstHeading)
                .expect_err("missing end marker should fail");

        assert!(error.to_string().contains("missing an end marker"));
    }

    #[test]
    fn sha256_text_is_stable() {
        assert_eq!(
            sha256_text("guardrails"),
            "5dc897f81321d2bafc2dc70f25356752d7cbd6f0b6cbd9e44b5f9969bc25042a"
        );
    }
}
