use super::WorkspaceState;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn open_workspace(path: &Path) -> WorkspaceState {
    let workspace = WorkspaceState::default();
    workspace.set_root(fs::canonicalize(path).unwrap()).unwrap();
    workspace
}

#[test]
fn directory_listing_shows_gitignored_files_and_respects_hidden_file_toggle() {
    let root = tempdir().unwrap();
    fs::write(
        root.path().join(".gitignore"),
        "ignored.txt\nignored-dir/\n",
    )
    .unwrap();
    fs::write(root.path().join(".env"), "TOKEN=local").unwrap();
    fs::write(root.path().join("visible.yaml"), "platform: web").unwrap();
    fs::write(root.path().join("ignored.txt"), "ignored").unwrap();
    fs::create_dir(root.path().join("ignored-dir")).unwrap();

    let workspace = open_workspace(root.path());
    let with_hidden = workspace.list_directory(Path::new("."), true).unwrap();
    let names: Vec<_> = with_hidden
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(names.contains(&".env"));
    assert!(names.contains(&".gitignore"));
    assert!(names.contains(&"visible.yaml"));
    assert!(names.contains(&"ignored.txt"));
    assert!(names.contains(&"ignored-dir"));

    let without_hidden = workspace.list_directory(Path::new("."), false).unwrap();
    assert!(!without_hidden.iter().any(|entry| entry.name == ".env"));
}

#[test]
fn workspace_rejects_parent_traversal_and_outside_paths() {
    let root = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    let workspace = open_workspace(root.path());

    assert!(workspace.read_file(Path::new("../secret.txt")).is_err());
    assert!(workspace
        .read_file(&outside.path().join("secret.txt"))
        .is_err());
    assert!(workspace
        .create_file(root.path(), "../escape.txt", "x")
        .is_err());
}

#[test]
fn relative_yaml_file_references_resolve_inside_workspace_only() {
    let root = tempdir().unwrap();
    fs::create_dir_all(root.path().join("flows/subflows")).unwrap();
    fs::create_dir_all(root.path().join("shared")).unwrap();
    fs::write(root.path().join("flows/subflows/main.yaml"), "---\n- wait: 1\n").unwrap();
    fs::write(root.path().join("shared/data.csv"), "id\n1\n").unwrap();
    let workspace = open_workspace(root.path());
    let source = root.path().join("flows/subflows/main.yaml");

    let resolved = workspace
        .resolve_file_reference(&source, "../../shared/data.csv")
        .unwrap();
    assert_eq!(resolved.file_name().unwrap(), "data.csv");
    assert!(workspace
        .resolve_file_reference(&source, "../../../../outside.txt")
        .is_err());
}

#[cfg(unix)]
#[test]
fn workspace_rejects_symlinks_that_escape_root() {
    use std::os::unix::fs::symlink;

    let root = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    symlink(outside.path(), root.path().join("external")).unwrap();
    let workspace = open_workspace(root.path());

    assert!(workspace
        .read_file(&root.path().join("external/secret.txt"))
        .is_err());
    assert!(workspace
        .remove_entry(&root.path().join("external"))
        .is_err());
}

#[test]
fn move_entry_rejects_overwrite_and_folder_self_nesting() {
    let root = tempdir().unwrap();
    fs::create_dir(root.path().join("folder")).unwrap();
    fs::create_dir(root.path().join("folder/child")).unwrap();
    fs::create_dir(root.path().join("target")).unwrap();
    fs::write(root.path().join("duplicate.txt"), "existing").unwrap();
    fs::write(root.path().join("folder/item.txt"), "item").unwrap();
    let workspace = open_workspace(root.path());

    assert!(workspace
        .move_entry(
            &root.path().join("folder"),
            &root.path().join("folder/child"),
            "folder",
        )
        .is_err());
    assert!(workspace
        .move_entry(
            &root.path().join("folder/item.txt"),
            root.path(),
            "duplicate.txt",
        )
        .is_err());

    let moved = workspace
        .move_entry(
            &root.path().join("folder/item.txt"),
            &root.path().join("target"),
            "item.txt",
        )
        .unwrap();
    assert!(moved.exists());
    assert!(!root.path().join("folder/item.txt").exists());
}
