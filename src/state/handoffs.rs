use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::diagnostics::{Diagnostic, DiagnosticReport};

use super::{
    HANDOFFS_DIR, is_kebab_case, now_timestamp, parse_toml_frontmatter, render_toml_frontmatter,
    title_from_slug,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffFrontmatter {
    pub id: u32,
    pub slug: String,
    pub title: String,
    pub status: HandoffStatus,
    pub created: String,
    pub updated: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_ids: Vec<u32>,
    pub template_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandoffRecord {
    pub path: String,
    pub frontmatter: HandoffFrontmatter,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandoffSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub status: HandoffStatus,
    pub created: String,
    pub updated: String,
    pub task_ids: Vec<String>,
    pub template_path: String,
    pub path: String,
}

#[derive(Debug, Default)]
pub struct HandoffCollection {
    pub handoffs: Vec<HandoffRecord>,
    pub diagnostics: DiagnosticReport,
}

impl HandoffRecord {
    pub fn new(id: u32, slug: &str, title: Option<&str>, task_ids: Vec<u32>, body: String) -> Self {
        let timestamp = now_timestamp();
        let title = title
            .map(str::to_string)
            .unwrap_or_else(|| title_from_slug(slug));
        Self {
            path: handoff_relative_path(id, slug),
            frontmatter: HandoffFrontmatter {
                id,
                slug: slug.to_string(),
                title,
                status: HandoffStatus::Open,
                created: timestamp.clone(),
                updated: timestamp,
                task_ids,
                template_path: "docs/project/handoff-template.md".to_string(),
            },
            body,
        }
    }

    pub fn id_string(&self) -> String {
        format!("{:04}", self.frontmatter.id)
    }

    pub fn absolute_path(&self, repo_root: &Path) -> PathBuf {
        repo_root.join(&self.path)
    }

    pub fn close(&mut self) -> Result<()> {
        if self.frontmatter.status == HandoffStatus::Closed {
            bail!("handoff {} is already closed", self.id_string());
        }
        self.frontmatter.status = HandoffStatus::Closed;
        self.frontmatter.updated = now_timestamp();
        Ok(())
    }

    pub fn write(&self, repo_root: &Path) -> Result<()> {
        let destination = self.absolute_path(repo_root);
        let rendered = render_toml_frontmatter(&self.frontmatter, &self.body)?;
        fs::write(&destination, rendered)
            .with_context(|| format!("failed to write {}", destination.display()))
    }

    pub fn summary(&self) -> HandoffSummary {
        HandoffSummary {
            id: self.id_string(),
            slug: self.frontmatter.slug.clone(),
            title: self.frontmatter.title.clone(),
            status: self.frontmatter.status,
            created: self.frontmatter.created.clone(),
            updated: self.frontmatter.updated.clone(),
            task_ids: self
                .frontmatter
                .task_ids
                .iter()
                .map(|id| format!("{id:04}"))
                .collect(),
            template_path: self.frontmatter.template_path.clone(),
            path: self.path.clone(),
        }
    }
}

pub fn load_collection(repo_root: &Path) -> Result<HandoffCollection> {
    let handoff_dir = repo_root.join(HANDOFFS_DIR);
    if !handoff_dir.exists() {
        return Ok(HandoffCollection::default());
    }
    let mut collection = HandoffCollection::default();

    for entry in fs::read_dir(&handoff_dir)
        .with_context(|| format!("failed to read {}", handoff_dir.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", handoff_dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }

        match parse_handoff(repo_root, &path) {
            Ok(handoff) => collection.handoffs.push(handoff),
            Err(error) => collection.diagnostics.push(Diagnostic::new(
                "handoff_parse_error",
                format!("failed to parse {}: {}", path.display(), error),
            )),
        }
    }

    collection
        .handoffs
        .sort_by_key(|handoff| handoff.frontmatter.id);
    collection.diagnostics.extend(
        validate_handoffs(repo_root, &collection.handoffs)
            .diagnostics()
            .to_vec(),
    );
    Ok(collection)
}

pub fn load_all(repo_root: &Path) -> Result<Vec<HandoffRecord>> {
    let collection = load_collection(repo_root)?;
    if collection.diagnostics.is_empty() {
        return Ok(collection.handoffs);
    }

    collection.diagnostics.print_stderr();
    bail!("handoff state is invalid; fix the records under `{HANDOFFS_DIR}` before continuing")
}

pub fn lint_handoffs(repo_root: &Path) -> Result<DiagnosticReport> {
    Ok(load_collection(repo_root)?.diagnostics)
}

pub fn next_handoff_id(handoffs: &[HandoffRecord]) -> u32 {
    handoffs
        .iter()
        .map(|handoff| handoff.frontmatter.id)
        .max()
        .unwrap_or(0)
        + 1
}

pub fn find_handoff(handoffs: &[HandoffRecord], id: u32) -> Result<HandoffRecord> {
    handoffs
        .iter()
        .find(|handoff| handoff.frontmatter.id == id)
        .cloned()
        .with_context(|| format!("handoff {:04} was not found", id))
}

pub fn handoff_relative_path(id: u32, slug: &str) -> String {
    format!("{HANDOFFS_DIR}/{id:04}-{slug}.md")
}

pub fn load_template(repo_root: &Path) -> Result<String> {
    let template_path = repo_root.join("docs/project/handoff-template.md");
    if template_path.exists() {
        return fs::read_to_string(&template_path)
            .with_context(|| format!("failed to read {}", template_path.display()));
    }

    Ok(include_str!("../../templates/shared/docs/project/handoff-template.md").to_string())
}

fn parse_handoff(repo_root: &Path, path: &Path) -> Result<HandoffRecord> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let (frontmatter, body): (HandoffFrontmatter, String) = parse_toml_frontmatter(&raw)?;
    if !is_kebab_case(&frontmatter.slug) {
        bail!(
            "handoff {} has an invalid slug `{}`",
            path.display(),
            frontmatter.slug
        );
    }

    let relative = path
        .strip_prefix(repo_root)
        .with_context(|| format!("failed to relativize {}", path.display()))?
        .to_string_lossy()
        .replace('\\', "/");
    let expected = handoff_relative_path(frontmatter.id, &frontmatter.slug);
    if relative != expected {
        bail!("{relative} should be named {expected} to match id/slug");
    }

    Ok(HandoffRecord {
        path: relative,
        frontmatter,
        body,
    })
}

fn validate_handoffs(repo_root: &Path, handoffs: &[HandoffRecord]) -> DiagnosticReport {
    let mut report = DiagnosticReport::default();
    let known_tasks = match crate::state::tasks::load_collection(repo_root) {
        Ok(collection) => collection
            .tasks
            .into_iter()
            .map(|task| task.frontmatter.id)
            .collect::<std::collections::BTreeSet<_>>(),
        Err(_) => return report,
    };

    for handoff in handoffs {
        if !repo_root.join(&handoff.frontmatter.template_path).exists() {
            report.push(Diagnostic::new(
                "handoff_template_missing",
                format!(
                    "{} references missing template path `{}`",
                    handoff.path, handoff.frontmatter.template_path
                ),
            ));
        }

        for task_id in &handoff.frontmatter.task_ids {
            if !known_tasks.contains(task_id) {
                report.push(Diagnostic::new(
                    "handoff_task_missing",
                    format!("{} references missing task {:04}", handoff.path, task_id),
                ));
            }
        }
    }

    report
}
