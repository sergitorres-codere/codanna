//! Profile system for project initialization

pub mod commands;
pub mod error;
pub mod fsops;
pub mod installer;
pub mod local;
pub mod lockfile;
pub mod manifest;
pub mod orchestrator;
pub mod project;
pub mod provider;
pub mod provider_registry;
pub mod reference;
pub mod resolver;
pub mod source_resolver;
pub mod template;
pub mod variables;
pub mod verification;

use error::ProfileResult;
use orchestrator::install_profile;
use provider::ProviderManifest;
use provider_registry::{ProviderRegistry, ProviderSource};
use reference::ProfileReference;
use source_resolver::resolve_profile_source;
use std::path::{Path, PathBuf};

/// Get the default profiles directory
/// Returns ~/.codanna/profiles/
pub fn profiles_dir() -> PathBuf {
    crate::init::global_dir().join("profiles")
}

/// Get the provider registry path
/// Returns ~/.codanna/providers.json
pub fn provider_registry_path() -> PathBuf {
    crate::init::global_dir().join("providers.json")
}

/// Initialize a profile to the current workspace
///
/// This is the public API for the `codanna profile init` command.
pub fn init_profile(profile_name: &str, source: Option<&Path>, force: bool) -> ProfileResult<()> {
    let workspace = std::env::current_dir()?;
    let profiles_dir = source.map(|p| p.to_path_buf()).unwrap_or_else(profiles_dir);

    if force {
        println!(
            "Installing profile '{profile_name}' with --force (will overwrite conflicting files)..."
        );
    } else {
        println!("Installing profile '{profile_name}' to workspace...");
    }

    install_profile(profile_name, &profiles_dir, &workspace, force)?;

    println!("\nProfile '{profile_name}' installed successfully");
    if force {
        println!("  Note: Conflicting files handled with --force");
        println!("  Use 'codanna profile verify {profile_name}' to check integrity");
    }

    Ok(())
}

/// Add a provider to the global registry
///
/// This is the public API for the `codanna profile provider add` command.
pub fn add_provider(source: &str, provider_id: Option<&str>) -> ProfileResult<()> {
    let registry_path = provider_registry_path();
    let mut registry = ProviderRegistry::load(&registry_path)?;

    // Parse source (GitHub shorthand, git URL, or local path)
    let provider_source = ProviderSource::parse(source);

    // Determine provider ID (user-specified or derive from source)
    let id = provider_id
        .map(String::from)
        .unwrap_or_else(|| derive_provider_id(&provider_source));

    // Check if already registered
    if registry.get_provider(&id).is_some() {
        println!("Provider '{id}' is already registered");
        println!("Use --force to update or remove it first");
        return Ok(());
    }

    // For now, we'll load the manifest from local path
    // TODO: Clone git repos to temp directory and load manifest
    let manifest = load_provider_manifest(&provider_source)?;

    // Add to registry
    registry.add_provider(id.clone(), &manifest, provider_source);
    registry.save(&registry_path)?;

    println!(
        "Added provider '{id}' ({} profiles available)",
        manifest.profiles.len()
    );
    for profile in &manifest.profiles {
        println!("  - {}", profile.name);
    }

    Ok(())
}

/// Remove a provider from the global registry
///
/// This is the public API for the `codanna profile provider remove` command.
pub fn remove_provider(provider_id: &str) -> ProfileResult<()> {
    let registry_path = provider_registry_path();
    let mut registry = ProviderRegistry::load(&registry_path)?;

    if registry.remove_provider(provider_id) {
        registry.save(&registry_path)?;
        println!("Removed provider '{provider_id}'");
    } else {
        println!("Provider '{provider_id}' not found");
    }

    Ok(())
}

/// List registered providers
///
/// This is the public API for the `codanna profile provider list` command.
pub fn list_providers(verbose: bool) -> ProfileResult<()> {
    let registry_path = provider_registry_path();
    let registry = ProviderRegistry::load(&registry_path)?;

    if registry.providers.is_empty() {
        println!("No providers registered");
        println!("\nAdd a provider with:");
        println!("  codanna profile provider add <source>");
        return Ok(());
    }

    println!("Registered providers:");
    for (id, provider) in &registry.providers {
        println!("\n{id}:");
        println!("  Name: {}", provider.name);
        match &provider.source {
            ProviderSource::Github { repo } => println!("  Source: github:{repo}"),
            ProviderSource::Git { url } => println!("  Source: {url}"),
            ProviderSource::Local { path } => println!("  Source: {path}"),
        }
        if verbose {
            println!("  Profiles ({}):", provider.profiles.len());
            for (name, info) in &provider.profiles {
                print!("    - {name} ({})", info.version);
                if let Some(desc) = &info.description {
                    print!(": {desc}");
                }
                println!();
            }
        } else {
            println!("  Profiles: {}", provider.profiles.len());
        }
    }

    Ok(())
}

/// Derive a provider ID from the source
fn derive_provider_id(source: &ProviderSource) -> String {
    match source {
        ProviderSource::Github { repo } => {
            // Extract last part: "codanna/claude-provider" → "claude-provider"
            repo.split('/').next_back().unwrap_or(repo).to_string()
        }
        ProviderSource::Git { url } => {
            // Extract repo name from URL
            url.trim_end_matches(".git")
                .split('/')
                .next_back()
                .unwrap_or("provider")
                .to_string()
        }
        ProviderSource::Local { path } => {
            // Use directory name
            Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("local")
                .to_string()
        }
    }
}

/// Load provider manifest from source
/// TODO: Support git cloning
fn load_provider_manifest(source: &ProviderSource) -> ProfileResult<ProviderManifest> {
    match source {
        ProviderSource::Local { path } => {
            let manifest_path = Path::new(path).join(".codanna-profile/provider.json");
            ProviderManifest::from_file(&manifest_path)
        }
        ProviderSource::Github { .. } | ProviderSource::Git { .. } => {
            // TODO: Clone to temp directory and load manifest
            Err(error::ProfileError::InvalidManifest {
                reason: "Git source providers not yet supported. Use local path for now."
                    .to_string(),
            })
        }
    }
}

/// Verify integrity of a specific profile
///
/// This is the public API for the `codanna profile verify` command.
pub fn verify_profile(profile_name: &str, verbose: bool) -> ProfileResult<()> {
    let workspace = std::env::current_dir()?;
    verification::verify_profile(&workspace, profile_name, verbose)
}

/// Verify all installed profiles
///
/// This is the public API for the `codanna profile verify --all` command.
pub fn verify_all_profiles(verbose: bool) -> ProfileResult<()> {
    let workspace = std::env::current_dir()?;
    verification::verify_all_profiles(&workspace, verbose)
}

/// Install profile from provider registry
///
/// Supports syntax:
/// - "myprofile" - searches all providers for profile
/// - "myprofile@provider" - installs from specific provider
///
/// This is the public API for registry-based installation.
pub fn install_profile_from_registry(profile_ref: &str, force: bool) -> ProfileResult<()> {
    let workspace = std::env::current_dir()?;

    // 1. Parse profile reference
    let reference = ProfileReference::parse(profile_ref);

    // 2. Load provider registry
    let registry_path = provider_registry_path();
    let registry = ProviderRegistry::load(&registry_path)?;

    if registry.providers.is_empty() {
        return Err(error::ProfileError::InvalidManifest {
            reason: "No providers registered. Add a provider first:\n  codanna profile provider add <source>".to_string(),
        });
    }

    // 3. Find provider
    let provider = match &reference.provider {
        Some(id) => {
            // Specific provider requested
            registry.get_provider(id).ok_or_else(|| {
                error::ProfileError::InvalidManifest {
                    reason: format!(
                        "Provider '{id}' not found\nUse 'codanna profile provider list' to see registered providers"
                    ),
                }
            })?
        }
        None => {
            // Search all providers for profile
            registry
                .find_provider_for_profile(&reference.profile)
                .ok_or_else(|| error::ProfileError::InvalidManifest {
                    reason: format!(
                        "Profile '{}' not found in any registered provider\nUse 'codanna profile provider list --verbose' to see available profiles",
                        reference.profile
                    ),
                })?
        }
    };

    // 4. Verify profile exists in provider
    if !provider.profiles.contains_key(&reference.profile) {
        return Err(error::ProfileError::InvalidManifest {
            reason: format!(
                "Profile '{}' not found in provider '{}'\nAvailable profiles: {}",
                reference.profile,
                provider.name,
                provider
                    .profiles
                    .keys()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }

    // 5. Resolve profile source
    println!(
        "Resolving profile '{}' from provider '{}'...",
        reference.profile, provider.name
    );
    let resolved = resolve_profile_source(&provider.source, &reference.profile)?;
    let profile_dir = resolved.profile_dir(&reference.profile);

    if !profile_dir.exists() {
        return Err(error::ProfileError::InvalidManifest {
            reason: format!("Profile directory not found: {}", profile_dir.display()),
        });
    }

    // 6. Install using atomic installer
    if force {
        println!(
            "Installing profile '{}' from provider '{}' with --force...",
            reference.profile, provider.name
        );
    } else {
        println!(
            "Installing profile '{}' from provider '{}'...",
            reference.profile, provider.name
        );
    }

    install_profile(
        &reference.profile,
        profile_dir.parent().unwrap(),
        &workspace,
        force,
    )?;

    println!("\nProfile '{}' installed successfully", reference.profile);
    if force {
        println!("  Note: Conflicting files handled with --force");
        println!(
            "  Use 'codanna profile verify {}' to check integrity",
            reference.profile
        );
    }

    Ok(())
}
