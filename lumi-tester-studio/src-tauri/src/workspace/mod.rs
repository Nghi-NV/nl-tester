mod commands;
#[cfg(test)]
mod tests;

use ignore::WalkBuilder;
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::RwLock;

#[derive(Default)]
pub struct WorkspaceState {
    root: RwLock<Option<PathBuf>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
}

impl WorkspaceState {
    fn root(&self) -> Result<PathBuf, String> {
        self.root
            .read()
            .map_err(|_| "Workspace state is unavailable".to_string())?
            .clone()
            .ok_or_else(|| "Open a workspace first".to_string())
    }

    fn set_root(&self, path: PathBuf) -> Result<(), String> {
        *self
            .root
            .write()
            .map_err(|_| "Workspace state is unavailable".to_string())? = Some(path);
        Ok(())
    }

    pub(crate) fn resolve_existing(&self, requested: &Path) -> Result<PathBuf, String> {
        let root = self.root()?;
        reject_parent_components(requested)?;
        let candidate = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            root.join(requested)
        };
        let canonical = fs::canonicalize(candidate).map_err(|error| error.to_string())?;
        ensure_inside(&root, &canonical)?;
        Ok(canonical)
    }

    fn resolve_file_reference(&self, source: &Path, reference: &str) -> Result<PathBuf, String> {
        let root = self.root()?;
        let source = self.resolve_existing(source)?;
        let referenced = Path::new(reference);
        let candidate = if referenced.is_absolute() {
            referenced.to_path_buf()
        } else {
            source
                .parent()
                .ok_or_else(|| "The source file has no parent folder".to_string())?
                .join(referenced)
        };
        let canonical = fs::canonicalize(candidate).map_err(|error| error.to_string())?;
        ensure_inside(&root, &canonical)?;
        if !canonical.is_file() {
            return Err("The referenced workspace path is not a file".to_string());
        }
        Ok(canonical)
    }

    fn resolve_new_entry(&self, requested: &Path) -> Result<PathBuf, String> {
        let root = self.root()?;
        reject_parent_components(requested)?;
        let candidate = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            root.join(requested)
        };
        ensure_inside(&root, &candidate)?;

        let parent = candidate
            .parent()
            .ok_or_else(|| "A file name is required".to_string())?;
        let canonical_parent = fs::canonicalize(parent).map_err(|error| error.to_string())?;
        ensure_inside(&root, &canonical_parent)?;
        let file_name = candidate
            .file_name()
            .ok_or_else(|| "A file name is required".to_string())?;
        Ok(canonical_parent.join(file_name))
    }

    fn resolve_entry(&self, requested: &Path) -> Result<PathBuf, String> {
        let root = self.root()?;
        reject_parent_components(requested)?;
        let candidate = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            root.join(requested)
        };
        let parent = candidate
            .parent()
            .ok_or_else(|| "A workspace entry is required".to_string())?;
        let canonical_parent = fs::canonicalize(parent).map_err(|error| error.to_string())?;
        ensure_inside(&root, &canonical_parent)?;
        let file_name = candidate
            .file_name()
            .ok_or_else(|| "A workspace entry is required".to_string())?;
        let path = canonical_parent.join(file_name);
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            let target = fs::canonicalize(&path).map_err(|error| error.to_string())?;
            ensure_inside(&root, &target)?;
        } else {
            let canonical = fs::canonicalize(&path).map_err(|error| error.to_string())?;
            ensure_inside(&root, &canonical)?;
        }
        Ok(path)
    }

    fn list_directory(
        &self,
        requested: &Path,
        show_hidden: bool,
    ) -> Result<Vec<WorkspaceEntry>, String> {
        let root = self.root()?;
        let directory = self.resolve_existing(requested)?;
        if !directory.is_dir() {
            return Err("The requested workspace path is not a directory".to_string());
        }

        let relative_depth = directory
            .strip_prefix(&root)
            .map_err(|_| "The requested workspace path is outside the open workspace".to_string())?
            .components()
            .count();
        let root_for_filter = root.clone();
        let directory_for_filter = directory.clone();
        let mut walker = WalkBuilder::new(&root);
        walker
            .hidden(!show_hidden)
            .git_ignore(false)
            .require_git(false)
            .git_exclude(false)
            .parents(false)
            .follow_links(false)
            .max_depth(Some(relative_depth + 1))
            .filter_entry(move |entry| {
                let path = entry.path();
                path == root_for_filter
                    || directory_for_filter.starts_with(path)
                    || path.parent() == Some(directory_for_filter.as_path())
            });

        let mut entries = Vec::new();
        for item in walker.build() {
            let entry = item.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path.parent() != Some(directory.as_path())
                || path.file_name().is_some_and(|name| name == ".git")
            {
                continue;
            }

            let metadata = match fs::symlink_metadata(path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            if metadata.file_type().is_symlink() {
                let Ok(target) = fs::canonicalize(path) else {
                    continue;
                };
                if !target.starts_with(&root) {
                    continue;
                }
            }

            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            entries.push(WorkspaceEntry {
                name: name.to_string(),
                path: path.to_string_lossy().into_owned(),
                is_directory: metadata.file_type().is_dir(),
            });
        }

        entries.sort_by(|left, right| {
            right
                .is_directory
                .cmp(&left.is_directory)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        Ok(entries)
    }

    #[cfg(test)]
    fn read_file(&self, path: &Path) -> Result<String, String> {
        let path = self.resolve_existing(path)?;
        fs::read_to_string(path).map_err(|error| error.to_string())
    }

    fn write_file(&self, path: &Path, content: &str) -> Result<(), String> {
        let path = self.resolve_existing(path)?;
        if !path.is_file() {
            return Err("The requested workspace path is not a file".to_string());
        }
        fs::write(path, content).map_err(|error| error.to_string())
    }

    fn create_file(&self, parent: &Path, name: &str, content: &str) -> Result<PathBuf, String> {
        validate_entry_name(name)?;
        let parent = self.resolve_existing(parent)?;
        if !parent.is_dir() {
            return Err("The destination is not a directory".to_string());
        }
        let path = self.resolve_new_entry(&parent.join(name))?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| error.to_string())?;
        use std::io::Write;
        file.write_all(content.as_bytes())
            .map_err(|error| error.to_string())?;
        Ok(path)
    }

    fn create_directory(&self, parent: &Path, name: &str) -> Result<PathBuf, String> {
        validate_entry_name(name)?;
        let parent = self.resolve_existing(parent)?;
        if !parent.is_dir() {
            return Err("The destination is not a directory".to_string());
        }
        let path = self.resolve_new_entry(&parent.join(name))?;
        fs::create_dir(&path).map_err(|error| error.to_string())?;
        Ok(path)
    }

    fn remove_entry(&self, requested: &Path) -> Result<(), String> {
        let root = self.root()?;
        let path = self.resolve_entry(requested)?;
        if path == root {
            return Err("The workspace root cannot be removed from the IDE".to_string());
        }
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_dir() {
            fs::remove_dir_all(path).map_err(|error| error.to_string())
        } else {
            fs::remove_file(path).map_err(|error| error.to_string())
        }
    }

    fn move_entry(
        &self,
        requested: &Path,
        destination: &Path,
        name: &str,
    ) -> Result<PathBuf, String> {
        validate_entry_name(name)?;
        let root = self.root()?;
        let source = self.resolve_entry(requested)?;
        if source == root {
            return Err("The workspace root cannot be moved".to_string());
        }
        let destination = self.resolve_existing(destination)?;
        if !destination.is_dir() {
            return Err("The destination is not a directory".to_string());
        }
        if source.is_dir() && destination.starts_with(&source) {
            return Err("A folder cannot be moved into itself".to_string());
        }

        let target = self.resolve_new_entry(&destination.join(name))?;
        if target.exists() {
            return Err("A file or folder with that name already exists".to_string());
        }
        fs::rename(source, &target).map_err(|error| error.to_string())?;
        Ok(target)
    }
}

fn reject_parent_components(path: &Path) -> Result<(), String> {
    if path
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err("Parent directory traversal is not allowed".to_string());
    }
    Ok(())
}

fn ensure_inside(root: &Path, candidate: &Path) -> Result<(), String> {
    if candidate.starts_with(root) {
        Ok(())
    } else {
        Err("The requested path is outside the open workspace".to_string())
    }
}

fn validate_entry_name(name: &str) -> Result<(), String> {
    let path = Path::new(name);
    if name.trim().is_empty()
        || name == "."
        || name == ".."
        || path.components().count() != 1
        || path.file_name().and_then(|value| value.to_str()) != Some(name)
    {
        return Err("Enter a valid file or folder name".to_string());
    }
    Ok(())
}

pub use commands::*;
