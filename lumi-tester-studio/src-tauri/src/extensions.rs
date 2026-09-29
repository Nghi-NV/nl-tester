use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const EXTENSION_SCHEMA_VERSION: u32 = 1;
const LUMI_EXTENSION_API_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_CONTRIBUTIONS_PER_KIND: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionManifest {
    pub schema_version: u32,
    pub lumi_api_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub publisher: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub contributes: ExtensionContributions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionContributions {
    #[serde(default)]
    pub docs: Vec<ExtensionDoc>,
    #[serde(default)]
    pub snippets: Vec<ExtensionSnippet>,
    #[serde(default)]
    pub templates: Vec<ExtensionTemplate>,
    #[serde(default)]
    pub selector_packs: Vec<ExtensionSelectorPack>,
    #[serde(default)]
    pub report_views: Vec<ExtensionReportView>,
    #[serde(default)]
    pub inspector_guides: Vec<ExtensionInspectorGuide>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionDoc {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionSnippet {
    pub prefix: String,
    pub description: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionTemplate {
    pub name: String,
    pub file_name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionSelectorPack {
    pub name: String,
    pub description: String,
    pub selectors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionReportView {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtensionInspectorGuide {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub selector_examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledExtension {
    pub manifest: ExtensionManifest,
    pub enabled: bool,
}

#[tauri::command]
pub fn list_lumi_extensions(app: AppHandle) -> Result<Vec<InstalledExtension>, String> {
    let directory = extension_directory(&app)?;
    let entries = fs::read_dir(&directory).map_err(|error| error.to_string())?;
    let mut extensions = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .path()
            .extension()
            .is_none_or(|extension| extension != "json")
        {
            continue;
        }
        extensions.push(read_installed(&entry.path())?);
    }
    extensions.sort_by(|left, right| {
        left.manifest
            .name
            .to_lowercase()
            .cmp(&right.manifest.name.to_lowercase())
    });
    Ok(extensions)
}

#[tauri::command]
pub fn install_lumi_extension(
    manifest_path: String,
    app: AppHandle,
) -> Result<InstalledExtension, String> {
    let source = PathBuf::from(&manifest_path);
    let metadata = fs::metadata(&source)
        .map_err(|error| format!("Could not open extension manifest: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_MANIFEST_BYTES {
        return Err("Choose a JSON extension manifest smaller than 1 MiB".to_string());
    }

    let input = fs::read_to_string(&source)
        .map_err(|error| format!("Could not read extension manifest: {error}"))?;
    let manifest: ExtensionManifest = serde_json::from_str(&input)
        .map_err(|error| format!("Invalid Lumi extension manifest: {error}"))?;
    validate_manifest(&manifest)?;

    let directory = extension_directory(&app)?;
    let path = manifest_path_in(&directory, &manifest.id);
    let enabled = if path.exists() {
        let current = read_installed(&path)?;
        if compare_versions(&manifest.version, &current.manifest.version)? != Ordering::Greater {
            return Err(format!(
                "{} {} is already installed; choose a newer version than {}",
                manifest.name, manifest.version, current.manifest.version
            ));
        }
        current.enabled
    } else {
        true
    };

    let installed = InstalledExtension { manifest, enabled };
    write_installed(&path, &installed)?;
    Ok(installed)
}

#[tauri::command]
pub fn set_lumi_extension_enabled(id: String, enabled: bool, app: AppHandle) -> Result<(), String> {
    validate_extension_id(&id)?;
    let directory = extension_directory(&app)?;
    let path = manifest_path_in(&directory, &id);
    let mut extension = read_installed(&path)?;
    extension.enabled = enabled;
    write_installed(&path, &extension)
}

#[tauri::command]
pub fn remove_lumi_extension(id: String, app: AppHandle) -> Result<(), String> {
    validate_extension_id(&id)?;
    let directory = extension_directory(&app)?;
    let path = manifest_path_in(&directory, &id);
    reject_symlink(&path)?;
    fs::remove_file(path).map_err(|error| error.to_string())
}

fn extension_directory(app: &AppHandle) -> Result<PathBuf, String> {
    let config = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&config).map_err(|error| error.to_string())?;
    let directory = config.join("lumi-extensions");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    reject_symlink(&directory)?;
    Ok(directory
        .canonicalize()
        .map_err(|error| error.to_string())?)
}

fn manifest_path_in(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.json"))
}

fn read_installed(path: &Path) -> Result<InstalledExtension, String> {
    reject_symlink(path)?;
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err("Installed extension manifest exceeds the 1 MiB limit".to_string());
    }
    let stored: InstalledExtension = serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("Could not read installed extension: {error}"))?,
    )
    .map_err(|error| format!("Installed extension data is invalid: {error}"))?;
    validate_manifest(&stored.manifest)?;
    if path.file_stem().and_then(|value| value.to_str()) != Some(stored.manifest.id.as_str()) {
        return Err("Installed extension ID does not match its manifest file".to_string());
    }
    Ok(stored)
}

fn write_installed(path: &Path, extension: &InstalledExtension) -> Result<(), String> {
    reject_symlink(path)?;
    let content = serde_json::to_vec_pretty(extension).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, content).map_err(|error| error.to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn reject_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err("Lumi extension storage cannot be a symbolic link".to_string())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn validate_manifest(manifest: &ExtensionManifest) -> Result<(), String> {
    validate_extension_id(&manifest.id)?;
    if manifest.schema_version != EXTENSION_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported extension manifest schema {}",
            manifest.schema_version
        ));
    }
    if manifest.lumi_api_version != LUMI_EXTENSION_API_VERSION {
        return Err(format!(
            "This extension targets Lumi extension API {}, but this IDE supports API {}",
            manifest.lumi_api_version, LUMI_EXTENSION_API_VERSION
        ));
    }
    if manifest.name.trim().is_empty() || manifest.name.len() > 120 {
        return Err("Extension name must contain 1 to 120 characters".to_string());
    }
    if manifest.publisher.len() > 120 || manifest.description.len() > 2000 {
        return Err("Extension publisher or description is too long".to_string());
    }
    compare_versions(&manifest.version, "0.0.0")?;
    let contributions = &manifest.contributes;
    if [
        contributions.docs.len(),
        contributions.snippets.len(),
        contributions.templates.len(),
        contributions.selector_packs.len(),
        contributions.report_views.len(),
        contributions.inspector_guides.len(),
    ]
    .into_iter()
    .any(|count| count > MAX_CONTRIBUTIONS_PER_KIND)
    {
        return Err("Each extension contribution type is limited to 256 entries".to_string());
    }
    for template in &contributions.templates {
        if template.name.trim().is_empty() || !is_safe_file_name(&template.file_name) {
            return Err(
                "Template names must be non-empty and template file names must not contain a path"
                    .to_string(),
            );
        }
    }
    for guide in &contributions.inspector_guides {
        if guide.name.trim().is_empty() || guide.name.len() > 120 || guide.description.len() > 2000
        {
            return Err(
                "Inspector guide names and descriptions are invalid or too long".to_string(),
            );
        }
        if guide.platforms.len() > 12
            || guide.selector_examples.len() > 128
            || guide.platforms.iter().any(|platform| platform.len() > 64)
            || guide
                .selector_examples
                .iter()
                .any(|selector| selector.trim().is_empty() || selector.len() > 512)
        {
            return Err(
                "Inspector guides may contain up to 12 platforms and 128 selector examples"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn validate_extension_id(id: &str) -> Result<(), String> {
    let bytes = id.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 64
        || !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit()
        || !bytes.iter().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(*byte, b'-' | b'_' | b'.')
        })
    {
        return Err("Extension ID must use lowercase letters, numbers, dots, underscores, or hyphens (maximum 64 characters)".to_string());
    }
    Ok(())
}

fn is_safe_file_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains(':')
        && name.len() <= 128
}

fn compare_versions(left: &str, right: &str) -> Result<Ordering, String> {
    fn parse(value: &str) -> Option<[u64; 3]> {
        let mut numbers = value.split('.').map(str::parse::<u64>);
        let parsed = [
            numbers.next()?.ok()?,
            numbers.next()?.ok()?,
            numbers.next()?.ok()?,
        ];
        numbers.next().is_none().then_some(parsed)
    }
    let left = parse(left)
        .ok_or_else(|| "Extension versions must use major.minor.patch format".to_string())?;
    let right = parse(right).ok_or_else(|| "Installed extension version is invalid".to_string())?;
    Ok(left.cmp(&right))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> ExtensionManifest {
        serde_json::from_str(
            r#"{
                "schemaVersion": 1,
                "lumiApiVersion": 1,
                "id": "team.login-flows",
                "name": "Login flows",
                "version": "1.2.3",
                "contributes": {
                    "snippets": [{ "prefix": "login", "description": "Login flow", "body": "- launchApp" }]
                }
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn accepts_compatible_declarative_manifest() {
        assert!(validate_manifest(&manifest()).is_ok());
    }

    #[test]
    fn rejects_unsafe_ids_instead_of_using_them_as_storage_paths() {
        let mut extension = manifest();
        extension.id = "../../outside".to_string();
        assert!(validate_manifest(&extension).is_err());
    }

    #[test]
    fn updates_require_a_newer_numeric_semver_version() {
        assert_eq!(
            compare_versions("1.10.0", "1.9.0").unwrap(),
            Ordering::Greater
        );
        assert_eq!(compare_versions("1.9.0", "1.10.0").unwrap(), Ordering::Less);
        assert!(compare_versions("latest", "1.0.0").is_err());
    }

    #[test]
    fn templates_cannot_write_outside_their_declared_file_name() {
        let mut extension = manifest();
        extension.contributes.templates.push(ExtensionTemplate {
            name: "escape".to_string(),
            file_name: "../../outside.yaml".to_string(),
            content: "- launchApp".to_string(),
        });
        assert!(validate_manifest(&extension).is_err());
    }
}
