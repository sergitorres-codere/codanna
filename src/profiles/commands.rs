//! CLI command definitions for profile management

use clap::Parser;
use std::path::PathBuf;

/// Profile management actions
#[derive(Debug, Clone, Parser)]
pub enum ProfileAction {
    /// Initialize project with a profile
    #[command(
        about = "Initialize project with a profile",
        after_help = "Examples:\n  codanna profile init claude\n  codanna profile init claude --source ~/.codanna/profiles"
    )]
    Init {
        /// Profile name to initialize
        profile_name: String,

        /// Profile source directory (defaults to ~/.codanna/profiles)
        #[arg(long)]
        source: Option<PathBuf>,

        /// Force initialization even if .codanna exists
        #[arg(short, long)]
        force: bool,
    },

    /// Install a profile to current workspace
    #[command(
        about = "Install a profile to current workspace",
        after_help = "Examples:\n  codanna profile install claude\n  codanna profile install claude --source git@github.com:codanna/profiles.git"
    )]
    Install {
        /// Profile name to install
        profile_name: String,

        /// Profile source (git URL or local directory)
        #[arg(long)]
        source: Option<String>,

        /// Git reference (branch, tag, or commit SHA)
        #[arg(long)]
        r#ref: Option<String>,

        /// Force installation even if profile exists
        #[arg(short, long)]
        force: bool,
    },

    /// List available profiles
    #[command(
        about = "List available profiles",
        after_help = "Example:\n  codanna profile list\n  codanna profile list --verbose"
    )]
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,

        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Show profile status for current workspace
    #[command(
        about = "Show active profile and installation status",
        after_help = "Example:\n  codanna profile status"
    )]
    Status {
        /// Show detailed file tracking information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Manage profile providers
    #[command(
        about = "Manage profile providers",
        after_help = "Examples:\n  codanna profile provider add codanna/claude-provider\n  codanna profile provider add ./my-provider --id custom\n  codanna profile provider list\n  codanna profile provider list --verbose\n  codanna profile provider remove claude-provider"
    )]
    Provider {
        #[command(subcommand)]
        action: ProviderAction,
    },

    /// Verify profile integrity
    #[command(
        about = "Verify profile integrity",
        after_help = "Examples:\n  codanna profile verify claude\n  codanna profile verify --all\n  codanna profile verify --all --verbose"
    )]
    Verify {
        /// Profile name to verify (omit for --all)
        profile_name: Option<String>,

        /// Verify all installed profiles
        #[arg(long, conflicts_with = "profile_name")]
        all: bool,

        /// Show detailed verification information
        #[arg(short, long)]
        verbose: bool,
    },
}

/// Provider management actions
#[derive(Debug, Clone, Parser)]
pub enum ProviderAction {
    /// Add a provider to the global registry
    #[command(
        about = "Add a provider to the global registry",
        after_help = "Examples:\n  codanna profile provider add codanna/claude-provider\n  codanna profile provider add https://github.com/codanna/profiles.git\n  codanna profile provider add ./my-provider --id custom"
    )]
    Add {
        /// Provider source (GitHub shorthand, git URL, or local path)
        source: String,

        /// Custom provider ID (defaults to derived from source)
        #[arg(long)]
        id: Option<String>,
    },

    /// Remove a provider from the global registry
    #[command(
        about = "Remove a provider from the global registry",
        after_help = "Example:\n  codanna profile provider remove claude-provider"
    )]
    Remove {
        /// Provider ID to remove
        provider_id: String,
    },

    /// List registered providers
    #[command(
        about = "List registered providers",
        after_help = "Examples:\n  codanna profile provider list\n  codanna profile provider list --verbose"
    )]
    List {
        /// Show detailed information including available profiles
        #[arg(short, long)]
        verbose: bool,
    },
}
