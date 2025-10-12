//! File system operations for plugin installation

use super::error::{PluginError, PluginResult};
use std::path::{Path, PathBuf};

/// Copy plugin files to destination with conflict detection
pub fn copy_plugin_files(
    source_dir: &Path,
    dest_dir: &Path,
    plugin_name: &str,
    file_list: &[String],
    force: bool,
) -> PluginResult<Vec<String>> {
    let mut copied_files = Vec::new();

    for file_path in file_list {
        let source_path = source_dir.join(file_path);
        let dest_path = calculate_dest_path(dest_dir, plugin_name, file_path);

        // Check for conflicts
        if dest_path.exists() && !force {
            // TODO: Query lockfile to find actual owner using lockfile.find_file_owner()
            // Currently hardcoded to "unknown" - should lookup which plugin installed this file
            return Err(PluginError::FileConflict {
                path: dest_path,
                owner: "unknown".to_string(),
            });
        }

        // Ensure parent directory exists
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Copy file
        std::fs::copy(&source_path, &dest_path)?;
        copied_files.push(dest_path.to_string_lossy().to_string());
    }

    Ok(copied_files)
}

/// Remove plugin files and clean up empty directories
pub fn remove_plugin_files(file_list: &[String]) -> PluginResult<()> {
    for file_path in file_list {
        let path = Path::new(file_path);
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        // Try to remove parent directory if empty
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir(parent); // Ignore errors if not empty
        }
    }

    Ok(())
}

/// Calculate destination path for a plugin file
fn calculate_dest_path(dest_dir: &Path, plugin_name: &str, file_path: &str) -> PathBuf {
    let path = Path::new(file_path);

    // Determine component type from path
    if file_path.starts_with("commands/") || file_path.starts_with("./commands/") {
        dest_dir
            .join(".claude/commands")
            .join(plugin_name)
            .join(path.file_name().unwrap())
    } else if file_path.starts_with("agents/") || file_path.starts_with("./agents/") {
        dest_dir
            .join(".claude/agents")
            .join(plugin_name)
            .join(path.file_name().unwrap())
    } else if file_path.starts_with("hooks/") || file_path.starts_with("./hooks/") {
        dest_dir
            .join(".claude/hooks")
            .join(plugin_name)
            .join(path.file_name().unwrap())
    } else {
        // Default to plugin-specific directory
        dest_dir
            .join(".claude/plugins")
            .join(plugin_name)
            .join(file_path)
    }
}

/// Verify file integrity using checksum
pub fn verify_file_integrity(file_path: &Path, expected_checksum: &str) -> PluginResult<bool> {
    use sha2::{Digest, Sha256};

    let content = std::fs::read(file_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    let result = hasher.finalize();
    let checksum = format!("{result:x}");

    Ok(checksum == expected_checksum)
}

/// Calculate checksum for a set of files
pub fn calculate_integrity(file_paths: &[String]) -> PluginResult<String> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();

    for file_path in file_paths {
        let path = Path::new(file_path);
        if path.exists() {
            let content = std::fs::read(path)?;
            hasher.update(&content);
            hasher.update(b"\n"); // Separator between files
        }
    }

    let result = hasher.finalize();
    Ok(format!("{result:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_calculate_dest_path() {
        let dest_dir = Path::new("/project");
        let plugin_name = "test-plugin";

        let cmd_path = calculate_dest_path(dest_dir, plugin_name, "commands/test.md");
        assert_eq!(
            cmd_path,
            PathBuf::from("/project/.claude/commands/test-plugin/test.md")
        );

        let agent_path = calculate_dest_path(dest_dir, plugin_name, "agents/helper.md");
        assert_eq!(
            agent_path,
            PathBuf::from("/project/.claude/agents/test-plugin/helper.md")
        );
    }

    #[test]
    fn test_copy_and_remove_files() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        let source_dir = temp_dir.path().join("source");
        let dest_dir = temp_dir.path().join("dest");

        // Create source files
        fs::create_dir_all(source_dir.join("commands"))?;
        fs::write(source_dir.join("commands/test.md"), "test content")?;

        // Copy files
        let files = vec!["commands/test.md".to_string()];
        let copied = copy_plugin_files(&source_dir, &dest_dir, "test-plugin", &files, false)?;

        assert_eq!(copied.len(), 1);
        assert!(
            dest_dir
                .join(".claude/commands/test-plugin/test.md")
                .exists()
        );

        // Remove files
        remove_plugin_files(&copied)?;
        assert!(
            !dest_dir
                .join(".claude/commands/test-plugin/test.md")
                .exists()
        );

        Ok(())
    }
}
