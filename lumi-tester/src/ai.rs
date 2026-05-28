use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;

const SKILL_FILES: &[&str] = &[
    "SKILL.md",
    "references/cli.csv",
    "references/command-catalog.md",
    "references/commands.csv",
    "references/debug-artifacts.md",
    "references/desktop.md",
    "references/patterns.md",
    "references/selector-discovery.md",
    "references/selectors.csv",
    "references/testcase-design.md",
    "scripts/lumi_agent.py",
    "agents/openai.yaml",
];

pub struct AiInstallOptions {
    pub repo: String,
    pub version: Option<String>,
    pub git_ref: String,
    pub ai_home: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub configure_codex: bool,
}

pub async fn install(options: AiInstallOptions) -> Result<()> {
    let home = dirs::home_dir().context("Could not resolve home directory")?;
    let version = normalize_version(
        options
            .version
            .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION"))),
    );
    let ai_home = options
        .ai_home
        .unwrap_or_else(|| home.join(".lumi-tester").join("ai"));
    let codex_home = options.codex_home.unwrap_or_else(|| home.join(".codex"));
    let target = detect_target()?;

    println!(
        "{}",
        "Installing Lumi Tester AI integration...".green().bold()
    );
    println!("  Repo: {}", options.repo.cyan());
    println!("  Version: {}", version.cyan());
    println!("  Target: {}", target.cyan());
    println!("  AI home: {}", ai_home.display().to_string().cyan());
    println!("  Codex home: {}", codex_home.display().to_string().cyan());

    install_mcp(&options.repo, &version, &target, &ai_home).await?;
    install_codex_skill(&options.repo, &version, &options.git_ref, &codex_home).await?;
    let snippets = write_config_snippets(&ai_home).await?;
    if options.configure_codex {
        configure_codex(&codex_home, &snippets.codex).await?;
    } else {
        println!("{} Skipped Codex config update", "•".blue());
    }

    println!();
    println!("{}", "Lumi Tester AI integration installed.".green().bold());
    println!("Restart Codex so it reloads the skill and MCP server.");
    println!("Quick checks:");
    println!("  lumi-tester doctor --platform android --json");
    println!("  node \"{}\"", snippets.server.display());

    Ok(())
}

fn normalize_version(version: String) -> String {
    if version == "latest" || version.starts_with('v') {
        version
    } else if version
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(false)
    {
        format!("v{}", version)
    } else {
        version
    }
}

fn detect_target() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let target = match (os, arch) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        _ => anyhow::bail!("Unsupported platform: {} {}", os, arch),
    };
    Ok(target.to_string())
}

async fn install_mcp(repo: &str, version: &str, target: &str, ai_home: &Path) -> Result<()> {
    let node = which::which("node").context("Missing required command: node")?;
    let npm = which::which("npm").context("Missing required command: npm")?;
    let package_dir = ai_home.join("mcp");
    let server_path = package_dir
        .join("node_modules")
        .join("lumi-tester-mcp")
        .join("src")
        .join("server.js");
    let asset = format!("lumi-tester-mcp-{}.tgz", target);
    let url = format!("{}/{}", release_base_url(repo, version), asset);
    let tmp_dir = std::env::temp_dir().join(format!("lumi-tester-ai-{}", Uuid::new_v4()));
    let tgz = tmp_dir.join(&asset);

    println!("{} Installing MCP package", "•".blue());
    println!("  Asset: {}", asset.cyan());
    tokio::fs::create_dir_all(&tmp_dir).await?;
    tokio::fs::create_dir_all(&package_dir).await?;
    download_to_file(&url, &tgz).await?;

    let status = Command::new(&npm)
        .arg("install")
        .arg("--prefix")
        .arg(&package_dir)
        .arg(&tgz)
        .arg("--omit=dev")
        .arg("--no-audit")
        .arg("--no-fund")
        .status()
        .await
        .context("Failed to run npm install for Lumi Tester MCP package")?;
    if !status.success() {
        anyhow::bail!("npm install failed with status {}", status);
    }

    if !server_path.is_file() {
        anyhow::bail!("MCP server was not installed at {}", server_path.display());
    }

    let node_status = Command::new(&node)
        .arg("--check")
        .arg(&server_path)
        .status()
        .await
        .context("Failed to validate MCP server with node --check")?;
    if !node_status.success() {
        anyhow::bail!("MCP server validation failed with status {}", node_status);
    }

    let _ = tokio::fs::remove_dir_all(&tmp_dir).await;
    println!("  Installed MCP server: {}", server_path.display());
    Ok(())
}

async fn install_codex_skill(
    repo: &str,
    version: &str,
    git_ref: &str,
    codex_home: &Path,
) -> Result<()> {
    let skill_dir = codex_home.join("skills").join("lumi-tester-agent");
    let base = format!(
        "{}/lumi-tester/ai/codex-skill/lumi-tester-agent",
        raw_base_url(repo, version, git_ref)
    );

    println!("{} Installing Codex skill", "•".blue());
    tokio::fs::create_dir_all(skill_dir.join("references")).await?;
    tokio::fs::create_dir_all(skill_dir.join("scripts")).await?;
    tokio::fs::create_dir_all(skill_dir.join("agents")).await?;

    for file in SKILL_FILES {
        let output = skill_dir.join(file);
        if let Some(parent) = output.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        download_to_file(&format!("{}/{}", base, file), &output).await?;
    }

    make_executable(&skill_dir.join("scripts").join("lumi_agent.py"))?;
    println!("  Installed Codex skill: {}", skill_dir.display());
    Ok(())
}

struct ConfigSnippets {
    codex: PathBuf,
    server: PathBuf,
}

async fn write_config_snippets(ai_home: &Path) -> Result<ConfigSnippets> {
    let lumi_bin = which::which("lumi-tester")
        .or_else(|_| std::env::current_exe())
        .context("Could not resolve lumi-tester binary path")?;
    let server_path = ai_home
        .join("mcp")
        .join("node_modules")
        .join("lumi-tester-mcp")
        .join("src")
        .join("server.js");
    let codex_snippet = ai_home.join("lumi-tester-mcp.codex.toml");
    let claude_snippet = ai_home.join("lumi-tester-mcp.claude.json");

    let codex = format!(
        "[mcp_servers.lumi-tester]\ncommand = \"node\"\nargs = [\"{}\"]\nenv = {{ LUMI_TESTER_BIN = \"{}\" }}\nstartup_timeout_sec = 10\ntool_timeout_sec = 300\n",
        toml_escape(&server_path.display().to_string()),
        toml_escape(&lumi_bin.display().to_string())
    );
    let claude = serde_json::json!({
        "mcpServers": {
            "lumi-tester": {
                "command": "node",
                "args": [server_path.display().to_string()],
                "env": {
                    "LUMI_TESTER_BIN": lumi_bin.display().to_string()
                }
            }
        }
    });

    tokio::fs::create_dir_all(ai_home).await?;
    tokio::fs::write(&codex_snippet, codex).await?;
    tokio::fs::write(
        &claude_snippet,
        serde_json::to_string_pretty(&claude)? + "\n",
    )
    .await?;

    println!("{} Wrote MCP config snippets", "•".blue());
    println!("  Codex: {}", codex_snippet.display());
    println!("  Claude: {}", claude_snippet.display());

    Ok(ConfigSnippets {
        codex: codex_snippet,
        server: server_path,
    })
}

async fn configure_codex(codex_home: &Path, snippet_path: &Path) -> Result<()> {
    let config = codex_home.join("config.toml");
    tokio::fs::create_dir_all(codex_home).await?;

    if config.is_file() {
        let current = tokio::fs::read_to_string(&config).await?;
        if current
            .lines()
            .any(|line| line.trim() == "[mcp_servers.lumi-tester]")
        {
            println!(
                "{} Codex MCP server already exists in {}",
                "•".blue(),
                config.display()
            );
            return Ok(());
        }

        let backup = config.with_extension(format!(
            "toml.bak-lumi-tester-{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        ));
        tokio::fs::copy(&config, &backup).await?;
        println!("  Backed up Codex config: {}", backup.display());
    }

    let snippet = tokio::fs::read_to_string(snippet_path).await?;
    let mut updated = if config.is_file() {
        tokio::fs::read_to_string(&config).await?
    } else {
        String::new()
    };
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push('\n');
    updated.push_str(&snippet);
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    tokio::fs::write(&config, updated).await?;

    println!(
        "{} Configured Codex MCP server in {}",
        "•".blue(),
        config.display()
    );
    Ok(())
}

async fn download_to_file(url: &str, output: &Path) -> Result<()> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to download {}", url))?
        .error_for_status()
        .with_context(|| format!("Download returned an error status: {}", url))?;
    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("Failed to read response body: {}", url))?;
    tokio::fs::write(output, bytes)
        .await
        .with_context(|| format!("Failed to write {}", output.display()))?;
    Ok(())
}

fn release_base_url(repo: &str, version: &str) -> String {
    if version == "latest" {
        format!("https://github.com/{}/releases/latest/download", repo)
    } else {
        format!("https://github.com/{}/releases/download/{}", repo, version)
    }
}

fn raw_base_url(repo: &str, version: &str, git_ref: &str) -> String {
    if version == "latest" {
        format!("https://raw.githubusercontent.com/{}/{}", repo, git_ref)
    } else {
        format!("https://raw.githubusercontent.com/{}/{}", repo, version)
    }
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}
