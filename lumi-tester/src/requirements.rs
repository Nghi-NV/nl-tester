//! SRS requirements coverage checking.
//!
//! Reads the `requirements/` format documented in the AI skill's
//! `testcase-design.md` ("SRS Requirements Format"): an `index.yaml` that
//! lists included module files, each holding a `requirements:` map. Cross
//! references every requirement id against `cases.csv` (the testcase matrix
//! the skill already asks agents to maintain) to compute - never trust a
//! hand-written status field - whether each requirement is actually backed
//! by a real, on-disk testcase file, or was explicitly marked `skip`ped.
//!
//! This exists because exploring a running app can only prove what is
//! CURRENTLY VISIBLE - it can never prove a requirement was silently never
//! built. Cross-checking against a real requirements source is the only way
//! to catch that gap, and computing it from files on disk (rather than a
//! self-reported status) is the only way the check can't be gamed by an
//! agent that forgot or chose not to write a testcase.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct IndexFile {
    #[serde(default)]
    policy: HashMap<String, Vec<String>>,
    common: Option<CommonField>,
    #[serde(default)]
    includes: Vec<String>,
}

/// `common:` accepts either a single path (the original shape) or a list of
/// paths - a project's cross-cutting `shared:` items commonly outgrow one
/// file (e.g. split by concern: `common_auth.yaml`, `common_hardware.yaml`).
/// Both shapes are only ever read here, never merged into anything
/// `check_coverage`'s gap logic uses - so this stays a tolerant, best-effort
/// parse (a missing file in the list is not an error, same as before).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CommonField {
    One(String),
    Many(Vec<String>),
}

impl CommonField {
    fn paths(&self) -> Vec<String> {
        match self {
            CommonField::One(s) => vec![s.clone()],
            CommonField::Many(v) => v.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ModuleFile {
    #[serde(default)]
    requirements: HashMap<String, Requirement>,
}

#[derive(Debug, Deserialize)]
struct Requirement {
    #[serde(default)]
    title: String,
    priority: Option<String>,
    skip: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CaseRow {
    #[serde(default)]
    requirement: String,
    #[serde(default)]
    tags: String,
    #[serde(default)]
    yaml: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RequirementStatus {
    pub id: String,
    pub title: String,
    pub priority: Option<String>,
    /// "skipped" | "covered" | "missing" | "incomplete"
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CoverageReport {
    pub total: usize,
    pub skipped: usize,
    pub covered: usize,
    pub gaps: Vec<RequirementStatus>,
    pub ok: bool,
}

/// Loads `index.yaml` at `index_path`, resolves `common` and every entry in
/// `includes` relative to the index file's own directory, and merges every
/// module's `requirements:` map into one id -> Requirement table.
fn load_requirements(index_path: &Path) -> Result<(IndexFile, HashMap<String, Requirement>)> {
    // `common`/`includes` paths are written relative to the PROJECT ROOT
    // (e.g. `requirements/auth.yaml`), matching how they read when a human
    // is looking at the project tree - not relative to index.yaml's own
    // directory, which would make every entry redundantly repeat
    // `requirements/`. index.yaml itself always lives at
    // `<project_root>/requirements/index.yaml`, so the project root is two
    // levels up from the index file.
    let base_dir = index_path
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let index_text = std::fs::read_to_string(index_path)
        .with_context(|| format!("failed to read {}", index_path.display()))?;
    let index: IndexFile = serde_yaml::from_str(&index_text)
        .with_context(|| format!("failed to parse {} as requirements index", index_path.display()))?;

    let mut merged: HashMap<String, Requirement> = HashMap::new();

    if let Some(common_field) = &index.common {
        for common_rel in common_field.paths() {
            let common_path = base_dir.join(&common_rel);
            // A common file only defines `shared:`, not `requirements:` - a
            // project without any cross-cutting items may omit `common:`
            // entirely, and any one path in a multi-file `common:` list may
            // not exist yet either, so a missing file here is not an error.
            if common_path.exists() {
                let _ = std::fs::read_to_string(&common_path)
                    .with_context(|| format!("failed to read {}", common_path.display()))?;
            }
        }
    }

    if index.includes.is_empty() {
        anyhow::bail!(
            "{} has no `includes` - nothing to check coverage for",
            index_path.display()
        );
    }

    for include_rel in &index.includes {
        let include_path = base_dir.join(include_rel);
        let text = std::fs::read_to_string(&include_path)
            .with_context(|| format!("failed to read included file {}", include_path.display()))?;
        let module: ModuleFile = serde_yaml::from_str(&text)
            .with_context(|| format!("failed to parse {} as a requirements module", include_path.display()))?;
        for (id, req) in module.requirements {
            if let Some(existing) = merged.insert(id.clone(), req) {
                anyhow::bail!(
                    "requirement id '{}' is defined more than once (last seen title: '{}') - ids must be unique across all included files",
                    id,
                    existing.title
                );
            }
        }
    }

    Ok((index, merged))
}

/// Reads `cases.csv` and returns, per requirement id, the union of `tags`
/// values across every row referencing it, plus whether at least one
/// referencing row's `yaml` path actually exists on disk. A row referencing
/// a requirement whose `yaml` file does NOT exist does not count as
/// coverage - a `cases.csv` row alone is a plan, not evidence a testcase
/// was actually written.
fn load_case_coverage(
    cases_csv: &Path,
    cases_csv_dir: &Path,
) -> Result<HashMap<String, HashSet<String>>> {
    let mut coverage: HashMap<String, HashSet<String>> = HashMap::new();
    let mut reader = csv::Reader::from_path(cases_csv)
        .with_context(|| format!("failed to read {}", cases_csv.display()))?;

    for record in reader.deserialize() {
        let row: CaseRow = record.with_context(|| format!("failed to parse a row in {}", cases_csv.display()))?;
        let req_id = row.requirement.trim();
        if req_id.is_empty() {
            continue;
        }
        let yaml_rel = row.yaml.trim();
        if yaml_rel.is_empty() {
            continue;
        }
        let yaml_path = cases_csv_dir.join(yaml_rel);
        if !yaml_path.exists() {
            continue;
        }
        let tags: HashSet<String> = row
            .tags
            .split(|c| c == ';' || c == ',' || c == '|')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        coverage.entry(req_id.to_string()).or_default().extend(tags);
    }

    Ok(coverage)
}

/// Computes the coverage report: every requirement is `skipped` (has a
/// `skip` reason), `covered` (has an on-disk testcase AND, if `policy`
/// defines required tags for its priority, all of them are present), or a
/// gap (`missing` - no covering testcase at all; `incomplete` - has a
/// testcase but is missing one or more policy-required tags).
pub fn check_coverage(index_path: &Path, cases_csv: Option<&Path>) -> Result<CoverageReport> {
    let (index, requirements) = load_requirements(index_path)?;

    let case_coverage: HashMap<String, HashSet<String>> = match cases_csv {
        Some(path) => {
            let dir = path.parent().unwrap_or_else(|| Path::new("."));
            load_case_coverage(path, dir)?
        }
        None => HashMap::new(),
    };

    let mut gaps = Vec::new();
    let mut skipped = 0usize;
    let mut covered = 0usize;
    let total = requirements.len();

    let mut ids: Vec<&String> = requirements.keys().collect();
    ids.sort();

    for id in ids {
        let req = &requirements[id];
        if let Some(reason) = req.skip.as_ref().filter(|s| !s.trim().is_empty()) {
            skipped += 1;
            let _ = reason;
            continue;
        }

        let tags = case_coverage.get(id);
        match tags {
            None => {
                gaps.push(RequirementStatus {
                    id: id.clone(),
                    title: req.title.clone(),
                    priority: req.priority.clone(),
                    status: "missing".to_string(),
                    detail: format!(
                        "no cases.csv row for '{}' references an existing yaml file",
                        id
                    ),
                });
            }
            Some(present_tags) => {
                let required = req
                    .priority
                    .as_ref()
                    .and_then(|p| index.policy.get(p))
                    .cloned()
                    .unwrap_or_default();
                let missing_tags: Vec<&String> =
                    required.iter().filter(|t| !present_tags.contains(*t)).collect();
                if missing_tags.is_empty() {
                    covered += 1;
                } else {
                    gaps.push(RequirementStatus {
                        id: id.clone(),
                        title: req.title.clone(),
                        priority: req.priority.clone(),
                        status: "incomplete".to_string(),
                        detail: format!(
                            "has a testcase, but priority '{}' requires tag(s) [{}] not found on any covering row",
                            req.priority.clone().unwrap_or_default(),
                            missing_tags
                                .iter()
                                .map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    });
                }
            }
        }
    }

    Ok(CoverageReport {
        total,
        skipped,
        covered,
        ok: gaps.is_empty(),
        gaps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        path
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lumi_req_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn skipped_requirement_never_counts_as_a_gap() {
        let dir = temp_dir();
        write(
            &dir,
            "requirements/index.yaml",
            "policy:\n  must: [smoke]\nincludes:\n  - requirements/auth.yaml\n",
        );
        write(
            &dir,
            "requirements/auth.yaml",
            "requirements:\n  REQ-AUTH-002:\n    title: Quen mat khau\n    priority: should\n    skip: \"not built yet\"\n",
        );

        let report = check_coverage(&dir.join("requirements/index.yaml"), None).unwrap();

        assert_eq!(report.total, 1);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.covered, 0);
        assert!(report.ok, "a skipped-only set must report ok");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn common_field_accepts_a_list_of_paths_not_just_one_string() {
        let dir = temp_dir();
        write(
            &dir,
            "requirements/index.yaml",
            "policy:\n  must: [smoke]\ncommon:\n  - requirements/common.yaml\n  - requirements/common_hardware.yaml\nincludes:\n  - requirements/auth.yaml\n",
        );
        write(&dir, "requirements/common.yaml", "shared:\n  error_toast:\n    targets: all\n    expect: [x]\n");
        // common_hardware.yaml deliberately left missing - a path in a
        // multi-file `common:` list not existing yet must not be an error,
        // same tolerance as the single-string form.
        write(
            &dir,
            "requirements/auth.yaml",
            "requirements:\n  REQ-AUTH-001:\n    title: Dang nhap\n    priority: must\n",
        );
        write(&dir, "smoke/login.yaml", "name: login\n---\n- wait: 1\n");
        write(
            &dir,
            "cases.csv",
            "id,requirement,tags,yaml\nc1,REQ-AUTH-001,smoke,smoke/login.yaml\n",
        );

        let report = check_coverage(
            &dir.join("requirements/index.yaml"),
            Some(&dir.join("cases.csv")),
        )
        .unwrap();

        assert!(report.ok);
        assert_eq!(report.covered, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn requirement_with_no_matching_case_row_is_a_missing_gap() {
        let dir = temp_dir();
        write(
            &dir,
            "requirements/index.yaml",
            "policy: {}\nincludes:\n  - requirements/auth.yaml\n",
        );
        write(
            &dir,
            "requirements/auth.yaml",
            "requirements:\n  REQ-AUTH-001:\n    title: Dang nhap\n    priority: must\n",
        );

        let report = check_coverage(&dir.join("requirements/index.yaml"), None).unwrap();

        assert_eq!(report.total, 1);
        assert!(!report.ok);
        assert_eq!(report.gaps.len(), 1);
        assert_eq!(report.gaps[0].status, "missing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn case_row_referencing_a_nonexistent_yaml_file_does_not_count_as_coverage() {
        let dir = temp_dir();
        write(
            &dir,
            "requirements/index.yaml",
            "policy: {}\nincludes:\n  - requirements/auth.yaml\n",
        );
        write(
            &dir,
            "requirements/auth.yaml",
            "requirements:\n  REQ-AUTH-001:\n    title: Dang nhap\n    priority: must\n",
        );
        write(
            &dir,
            "cases.csv",
            "id,requirement,tags,yaml\nc1,REQ-AUTH-001,smoke,smoke/does_not_exist.yaml\n",
        );

        let report = check_coverage(
            &dir.join("requirements/index.yaml"),
            Some(&dir.join("cases.csv")),
        )
        .unwrap();

        assert!(!report.ok);
        assert_eq!(report.gaps[0].status, "missing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn covered_requires_policy_tags_present_not_just_any_testcase() {
        let dir = temp_dir();
        write(
            &dir,
            "requirements/index.yaml",
            "policy:\n  must: [smoke, negative]\nincludes:\n  - requirements/auth.yaml\n",
        );
        write(
            &dir,
            "requirements/auth.yaml",
            "requirements:\n  REQ-AUTH-001:\n    title: Dang nhap\n    priority: must\n",
        );
        write(&dir, "smoke/login.yaml", "name: login\n---\n- wait: 1\n");
        write(
            &dir,
            "cases.csv",
            "id,requirement,tags,yaml\nc1,REQ-AUTH-001,smoke,smoke/login.yaml\n",
        );

        let report = check_coverage(
            &dir.join("requirements/index.yaml"),
            Some(&dir.join("cases.csv")),
        )
        .unwrap();

        assert!(!report.ok, "must-priority requires smoke AND negative - only smoke was provided");
        assert_eq!(report.gaps[0].status, "incomplete");
        assert!(report.gaps[0].detail.contains("negative"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fully_covered_requirement_reports_ok() {
        let dir = temp_dir();
        write(
            &dir,
            "requirements/index.yaml",
            "policy:\n  must: [smoke, negative]\nincludes:\n  - requirements/auth.yaml\n",
        );
        write(
            &dir,
            "requirements/auth.yaml",
            "requirements:\n  REQ-AUTH-001:\n    title: Dang nhap\n    priority: must\n",
        );
        write(&dir, "smoke/login.yaml", "name: login\n---\n- wait: 1\n");
        write(&dir, "smoke/login_negative.yaml", "name: login_negative\n---\n- wait: 1\n");
        write(
            &dir,
            "cases.csv",
            "id,requirement,tags,yaml\nc1,REQ-AUTH-001,smoke,smoke/login.yaml\nc2,REQ-AUTH-001,negative,smoke/login_negative.yaml\n",
        );

        let report = check_coverage(
            &dir.join("requirements/index.yaml"),
            Some(&dir.join("cases.csv")),
        )
        .unwrap();

        assert!(report.ok);
        assert_eq!(report.covered, 1);
        assert!(report.gaps.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_requirement_id_across_included_files_is_rejected() {
        let dir = temp_dir();
        write(
            &dir,
            "requirements/index.yaml",
            "policy: {}\nincludes:\n  - requirements/a.yaml\n  - requirements/b.yaml\n",
        );
        write(
            &dir,
            "requirements/a.yaml",
            "requirements:\n  REQ-001:\n    title: A\n    priority: must\n",
        );
        write(
            &dir,
            "requirements/b.yaml",
            "requirements:\n  REQ-001:\n    title: B duplicate\n    priority: must\n",
        );

        let result = check_coverage(&dir.join("requirements/index.yaml"), None);

        assert!(result.is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
