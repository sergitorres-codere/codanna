//! Plugin management system for Claude Code plugins
//!
//! This module provides functionality for installing, updating, and managing
//! Claude Code plugins from Git-based marketplaces.

pub mod error;
pub mod fsops;
pub mod lockfile;
pub mod marketplace;
pub mod merger;
pub mod plugin;
pub mod resolver;

use crate::Settings;
use chrono::Utc;
use error::{PluginError, PluginResult};
use fsops::{calculate_integrity, copy_plugin_files, copy_plugin_payload};
use lockfile::{PluginLockEntry, PluginLockfile};
use marketplace::MarketplaceManifest;
use plugin::{HookSpec, PathSpec, PluginManifest};
use resolver::{clone_repository, extract_subdirectory};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct WorkspacePaths {
    root: PathBuf,
    commands_dir: PathBuf,
    agents_dir: PathBuf,
    hooks_dir: PathBuf,
    plugins_dir: PathBuf,
    lockfile_path: PathBuf,
    mcp_path: PathBuf,
}

impl WorkspacePaths {
    fn for_root(root: PathBuf) -> Self {
        let claude_dir = root.join(".claude");
        let commands_dir = claude_dir.join("commands");
        let agents_dir = claude_dir.join("agents");
        let hooks_dir = claude_dir.join("hooks");
        let plugins_dir = claude_dir.join("plugins");
        let lockfile_path = root.join(".codanna/plugins/lockfile.json");
        let mcp_path = root.join(".mcp.json");

        Self {
            root,
            commands_dir,
            agents_dir,
            hooks_dir,
            plugins_dir,
            lockfile_path,
            mcp_path,
        }
    }
}

/// Install a plugin from a marketplace
pub fn add_plugin(
    settings: &Settings,
    marketplace_url: &str,
    plugin_name: &str,
    git_ref: Option<&str>,
    force: bool,
    dry_run: bool,
) -> Result<(), PluginError> {
    let workspace_root = resolve_workspace_root(settings)?;

    if dry_run {
        println!("DRY RUN: Would install plugin '{plugin_name}' from {marketplace_url}");
        if let Some(r) = git_ref {
            println!("  Using ref: {r}");
        }
        if force {
            println!("  Force mode: would overwrite conflicts");
        }
        println!("  Target workspace: {}", workspace_root.display());
        return Ok(());
    }

    let paths = WorkspacePaths::for_root(workspace_root.clone());
    ensure_workspace_layout(&paths)?;

    let mut lockfile = load_lockfile(&paths)?;

    if let Some(existing) = lockfile.get_plugin(plugin_name) {
        if !force {
            return Err(PluginError::AlreadyInstalled {
                name: plugin_name.to_string(),
                version: existing.version.clone(),
            });
        }
    }

    let marketplace_dir = tempdir()?;
    let commit_sha = clone_repository(marketplace_url, marketplace_dir.path(), git_ref)?;

    let marketplace_manifest_path = marketplace_dir
        .path()
        .join(".claude-plugin/marketplace.json");
    let marketplace_manifest = MarketplaceManifest::from_file(&marketplace_manifest_path)?;
    let plugin_entry = marketplace_manifest
        .find_plugin(plugin_name)
        .ok_or_else(|| PluginError::PluginNotFound {
            name: plugin_name.to_string(),
        })?;

    let plugin_source = normalize_source_path(&plugin_entry.source);
    let plugin_dir = tempdir()?;
    extract_subdirectory(marketplace_dir.path(), &plugin_source, plugin_dir.path())?;

    let plugin_manifest_path = plugin_dir.path().join(".claude-plugin/plugin.json");
    let plugin_manifest = PluginManifest::from_file(&plugin_manifest_path)?;

    let component_files = collect_component_files(plugin_dir.path(), &plugin_manifest)?;
    if settings.debug {
        eprintln!("DEBUG: component files for plugin '{plugin_name}': {component_files:?}");
    }

    let mut copied_files = Vec::new();

    if !component_files.is_empty() {
        let component_paths = copy_plugin_files(
            plugin_dir.path(),
            &paths.root,
            plugin_name,
            &component_files,
            force,
        )?;
        copied_files.extend(component_paths);
    }

    let payload_paths = copy_plugin_payload(plugin_dir.path(), &paths.root, plugin_name, force)?;
    copied_files.extend(payload_paths);

    let mut mcp_keys = Vec::new();
    if let Some(mcp_servers) = load_plugin_mcp(plugin_dir.path(), &plugin_manifest)? {
        let project_mcp_path = &paths.mcp_path;
        let added = merger::merge_mcp_servers(project_mcp_path, &mcp_servers, plugin_name, force)?;
        if !added.is_empty() {
            mcp_keys = added;
            copied_files.push(project_mcp_path.to_string_lossy().replace('\\', "/"));
        }
    }

    let copied_files = normalize_paths(&paths.root, copied_files);
    let integrity_inputs = to_absolute_paths(&paths, &copied_files);
    let integrity = calculate_integrity(&integrity_inputs)?;
    let timestamp = Utc::now().to_rfc3339();

    let entry = PluginLockEntry {
        name: plugin_name.to_string(),
        version: plugin_manifest.version.clone(),
        commit: commit_sha,
        marketplace_url: marketplace_url.to_string(),
        installed_at: timestamp.clone(),
        updated_at: timestamp,
        integrity,
        files: copied_files,
        mcp_keys,
    };

    lockfile.add_plugin(entry);
    save_lockfile(&paths, &lockfile)?;

    println!(
        "Plugin '{plugin_name}' installed into {}",
        paths.root.display()
    );
    Ok(())
}

/// Remove an installed plugin
pub fn remove_plugin(
    settings: &Settings,
    plugin_name: &str,
    force: bool,
    dry_run: bool,
) -> Result<(), PluginError> {
    let workspace_root = resolve_workspace_root(settings)?;
    let paths = WorkspacePaths::for_root(workspace_root.clone());

    if dry_run {
        println!("DRY RUN: Would remove plugin '{plugin_name}'");
        if force {
            println!("  Force mode: would ignore dependencies");
        }
        println!("  Target workspace: {}", paths.root.display());
        return Ok(());
    }

    let mut lockfile = load_lockfile(&paths)?;

    let entry = match lockfile.get_plugin(plugin_name) {
        Some(entry) => entry.clone(),
        None => {
            return Err(PluginError::NotInstalled {
                name: plugin_name.to_string(),
            });
        }
    };

    // TODO: Consider dependency graph when available. For now we ignore `force`.

    let absolute_files = to_absolute_paths(&paths, &entry.files);
    fsops::remove_plugin_files(&absolute_files)?;

    let payload_dir = paths.plugins_dir.join(plugin_name);
    if payload_dir.exists() {
        let _ = fs::remove_dir_all(&payload_dir);
    }

    if !entry.mcp_keys.is_empty() {
        merger::remove_mcp_servers(&paths.mcp_path, &entry.mcp_keys)?;
    }

    lockfile.remove_plugin(plugin_name);
    save_lockfile(&paths, &lockfile)?;

    println!(
        "Removed plugin '{plugin_name}' from {}",
        paths.root.display()
    );
    Ok(())
}

/// Update an installed plugin
pub fn update_plugin(
    settings: &Settings,
    plugin_name: &str,
    git_ref: Option<&str>,
    force: bool,
    dry_run: bool,
) -> Result<(), PluginError> {
    let workspace_root = resolve_workspace_root(settings)?;

    if dry_run {
        println!("DRY RUN: Would update plugin '{plugin_name}'");
        if let Some(r) = git_ref {
            println!("  To ref: {r}");
        }
        if force {
            println!("  Force mode: would overwrite local changes");
        }
        println!("  Target workspace: {}", workspace_root.display());
        return Ok(());
    }

    // TODO: Implement actual plugin update
    println!(
        "Updating plugin '{plugin_name}' in {}",
        workspace_root.display()
    );

    // 1. Read lockfile to get marketplace URL
    // 2. Fetch new version
    // 3. Compute diff
    // 4. Apply changes
    // 5. Update lockfile

    Ok(())
}

/// List installed plugins
pub fn list_plugins(settings: &Settings, verbose: bool, json: bool) -> Result<(), PluginError> {
    let workspace_root = resolve_workspace_root(settings)?;
    let paths = WorkspacePaths::for_root(workspace_root.clone());
    let lockfile = load_lockfile(&paths)?;

    let mut entries: Vec<_> = lockfile.plugins.values().cloned().collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    if json {
        let payload = serde_json::json!({
            "workspace": paths.root,
            "plugins": entries,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if entries.is_empty() {
        println!("No plugins installed in workspace {}", paths.root.display());
        if verbose {
            println!("\nUse 'codanna plugin add <marketplace> <plugin>' to install a plugin");
        }
    } else {
        println!("Plugins in workspace {}:", paths.root.display());
        for entry in entries {
            println!(
                "  - {} @ {} (commit {})",
                entry.name, entry.version, entry.commit
            );
            if verbose {
                println!("    source: {}", entry.marketplace_url);
                println!("    files: {}", entry.files.len());
            }
        }
    }
    Ok(())
}

/// Verify integrity of a specific plugin
pub fn verify_plugin(
    settings: &Settings,
    plugin_name: &str,
    verbose: bool,
) -> Result<(), PluginError> {
    let workspace_root = resolve_workspace_root(settings)?;
    let paths = WorkspacePaths::for_root(workspace_root.clone());
    let lockfile = load_lockfile(&paths)?;

    let entry = match lockfile.get_plugin(plugin_name) {
        Some(entry) => entry,
        None => {
            return Err(PluginError::NotInstalled {
                name: plugin_name.to_string(),
            });
        }
    };

    if verbose {
        println!(
            "Verifying plugin '{plugin_name}' in workspace {}...",
            paths.root.display()
        );
        println!("  Stored integrity: {}", entry.integrity);
    }

    verify_entry(&paths, entry, verbose)?;

    println!("Plugin '{plugin_name}' verified successfully");
    Ok(())
}

/// Verify all installed plugins
pub fn verify_all_plugins(settings: &Settings, verbose: bool) -> Result<(), PluginError> {
    let workspace_root = resolve_workspace_root(settings)?;
    let paths = WorkspacePaths::for_root(workspace_root.clone());
    let lockfile = load_lockfile(&paths)?;

    if lockfile.plugins.is_empty() {
        if verbose {
            println!("No plugins installed in workspace {}", paths.root.display());
        }
        return Ok(());
    }

    for entry in lockfile.plugins.values() {
        verify_entry(&paths, entry, verbose)?;
    }

    println!("All plugins verified successfully");
    Ok(())
}

fn ensure_workspace_layout(paths: &WorkspacePaths) -> PluginResult<()> {
    for sub in [
        &paths.commands_dir,
        &paths.agents_dir,
        &paths.hooks_dir,
        &paths.plugins_dir,
    ] {
        fs::create_dir_all(sub)?;
    }
    fs::create_dir_all(paths.lockfile_path.parent().unwrap())?;
    Ok(())
}

fn load_lockfile(paths: &WorkspacePaths) -> PluginResult<PluginLockfile> {
    PluginLockfile::load(&paths.lockfile_path)
}

fn save_lockfile(paths: &WorkspacePaths, lockfile: &PluginLockfile) -> PluginResult<()> {
    lockfile.save(&paths.lockfile_path)
}

fn to_absolute_paths(paths: &WorkspacePaths, files: &[String]) -> Vec<String> {
    files
        .iter()
        .map(|relative| {
            paths
                .root
                .join(relative)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

fn verify_entry(
    paths: &WorkspacePaths,
    entry: &PluginLockEntry,
    verbose: bool,
) -> PluginResult<()> {
    let absolute_files = to_absolute_paths(paths, &entry.files);
    let actual = calculate_integrity(&absolute_files)?;

    if actual != entry.integrity {
        return Err(PluginError::IntegrityCheckFailed {
            plugin: entry.name.clone(),
            expected: entry.integrity.clone(),
            actual,
        });
    }

    if verbose {
        println!(
            "  Integrity OK for '{}' ({} files)",
            entry.name,
            entry.files.len()
        );
    }

    if !entry.mcp_keys.is_empty() && paths.mcp_path.exists() {
        let content = fs::read_to_string(&paths.mcp_path)?;
        let json: Value = serde_json::from_str(&content)?;
        let servers = json
            .get("mcpServers")
            .and_then(|value| value.as_object())
            .cloned()
            .unwrap_or_default();

        for key in &entry.mcp_keys {
            if !servers.contains_key(key) {
                return Err(PluginError::IntegrityCheckFailed {
                    plugin: entry.name.clone(),
                    expected: format!("mcp server '{key}' present"),
                    actual: "missing".to_string(),
                });
            }
        }

        if verbose {
            println!(
                "  MCP servers verified for '{}': {:?}",
                entry.name, entry.mcp_keys
            );
        }
    }

    Ok(())
}

fn normalize_source_path(source: &str) -> String {
    let trimmed = source.trim();
    let without_prefix = trimmed.trim_start_matches("./");
    let without_slash = without_prefix.trim_start_matches('/');
    if without_slash.is_empty() {
        ".".to_string()
    } else {
        without_slash.to_string()
    }
}

fn collect_component_files(
    plugin_root: &Path,
    manifest: &PluginManifest,
) -> PluginResult<Vec<String>> {
    let mut files = HashSet::new();

    add_directory_files(plugin_root, "commands", &mut files)?;
    if let Some(spec) = &manifest.commands {
        add_spec_paths(plugin_root, spec, &mut files)?;
    }

    add_directory_files(plugin_root, "agents", &mut files)?;
    if let Some(spec) = &manifest.agents {
        add_spec_paths(plugin_root, spec, &mut files)?;
    }

    add_directory_files(plugin_root, "hooks", &mut files)?;
    if let Some(HookSpec::Path(path)) = &manifest.hooks {
        add_single_path(plugin_root, path, &mut files)?;
    }

    let mut list: Vec<_> = files.into_iter().collect();
    list.sort();
    Ok(list)
}

fn add_directory_files(
    plugin_root: &Path,
    directory: &str,
    files: &mut HashSet<String>,
) -> PluginResult<()> {
    let dir_path = plugin_root.join(directory);
    if !dir_path.exists() {
        return Ok(());
    }

    for file in collect_files_for_path(plugin_root, &dir_path)? {
        files.insert(file);
    }
    Ok(())
}

fn add_spec_paths(
    plugin_root: &Path,
    spec: &PathSpec,
    files: &mut HashSet<String>,
) -> PluginResult<()> {
    match spec {
        PathSpec::Single(path) => add_single_path(plugin_root, path, files)?,
        PathSpec::Multiple(paths) => {
            for path in paths {
                add_single_path(plugin_root, path, files)?;
            }
        }
    }
    Ok(())
}

fn add_single_path(
    plugin_root: &Path,
    path: &str,
    files: &mut HashSet<String>,
) -> PluginResult<()> {
    let sanitized = sanitize_manifest_path(path);
    if sanitized == "." {
        return Err(PluginError::InvalidPluginManifest {
            reason: format!("Referenced path '{path}' must not point to plugin root"),
        });
    }
    let full_path = plugin_root.join(&sanitized);
    if !full_path.exists() {
        return Err(PluginError::InvalidPluginManifest {
            reason: format!("Referenced path '{path}' does not exist"),
        });
    }

    for file in collect_files_for_path(plugin_root, &full_path)? {
        files.insert(file);
    }
    Ok(())
}

fn collect_files_for_path(base: &Path, target: &Path) -> PluginResult<Vec<String>> {
    if target.is_file() {
        let rel = target.strip_prefix(base).unwrap_or(target).to_path_buf();
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        return Ok(vec![rel_str]);
    }

    if target.is_dir() {
        let mut files = Vec::new();
        for entry in WalkDir::new(target).into_iter() {
            let entry = entry.map_err(|e| PluginError::IoError(io::Error::other(e)))?;
            if entry.file_type().is_dir() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(base)
                .unwrap_or(entry.path())
                .to_path_buf();
            files.push(rel.to_string_lossy().replace('\\', "/"));
        }
        return Ok(files);
    }

    Ok(Vec::new())
}

fn sanitize_manifest_path(path: &str) -> String {
    let trimmed = path.trim();
    let without_prefix = trimmed.trim_start_matches("./");
    if without_prefix.is_empty() {
        ".".to_string()
    } else {
        without_prefix.to_string()
    }
}

fn load_plugin_mcp(plugin_root: &Path, manifest: &PluginManifest) -> PluginResult<Option<Value>> {
    if let Some(spec) = &manifest.mcp_servers {
        return merger::load_plugin_mcp_servers(plugin_root, spec).map(Some);
    }

    let default_mcp = plugin_root.join(".mcp.json");
    if default_mcp.exists() {
        let content = fs::read_to_string(&default_mcp)?;
        let json: Value = serde_json::from_str(&content)?;
        if let Some(servers) = json.get("mcpServers") {
            if servers.is_object() {
                return Ok(Some(servers.clone()));
            }
        }
    }

    Ok(None)
}

fn normalize_paths(workspace_root: &Path, files: Vec<String>) -> Vec<String> {
    let mut unique = HashSet::new();
    for file in files {
        let path = PathBuf::from(&file);
        let rel = path
            .strip_prefix(workspace_root)
            .unwrap_or(&path)
            .to_path_buf();
        unique.insert(rel.to_string_lossy().replace('\\', "/"));
    }
    let mut list: Vec<_> = unique.into_iter().collect();
    list.sort();
    list
}

fn resolve_workspace_root(settings: &Settings) -> Result<PathBuf, PluginError> {
    if let Some(root) = &settings.workspace_root {
        if root.is_absolute() {
            Ok(root.clone())
        } else {
            let cwd = std::env::current_dir()?;
            Ok(cwd.join(root))
        }
    } else {
        Ok(std::env::current_dir()?)
    }
}
