//! CLI entry point for the codebase intelligence system.
//!
//! Provides commands for indexing, querying, and serving code intelligence data.
//! Main components: Cli parser, Commands enum, and async runtime with MCP server support.

use clap::{
    Parser, Subcommand,
    builder::styling::{AnsiColor, Effects, Styles},
};
use codanna::FileId;
use codanna::parsing::{LanguageParser, PythonParser, RustParser};
use codanna::{IndexPersistence, RelationKind, Settings, SimpleIndexer, Symbol, SymbolKind};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

fn clap_cargo_style() -> Styles {
    Styles::styled()
        .header(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .usage(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

/// Create custom help text with consistent styling
fn create_custom_help() -> String {
    use codanna::display::theme::Theme;
    use console::style;

    let mut help = String::new();

    // Quick Start section
    if Theme::should_disable_colors() {
        help.push_str("Quick Start:\n");
    } else {
        help.push_str(&format!("{}\n", style("Quick Start:").cyan().bold()));
    }
    help.push_str("  $ codanna init              # Set up in current directory\n");
    help.push_str("  $ codanna index src         # Index your source code\n");
    help.push_str("  $ codanna mcp-test          # Verify Claude can connect\n\n");

    // About section
    help.push_str(
        "Codanna provides AI assistants like Claude with deep understanding of your codebase.\n\n",
    );
    help.push_str("Fast parallel indexing with natural language search capabilities.\n\n");

    // Usage
    if Theme::should_disable_colors() {
        help.push_str("Usage:");
    } else {
        help.push_str(&format!("{}", style("Usage:").cyan().bold()));
    }
    help.push_str(" codanna [OPTIONS] <COMMAND>\n\n");

    // Commands
    if Theme::should_disable_colors() {
        help.push_str("Commands:\n");
    } else {
        help.push_str(&format!("{}\n", style("Commands:").cyan().bold()));
    }
    help.push_str("  init        Set up .codanna directory with default configuration\n");
    help.push_str(
        "  index       Build searchable index from your codebase with fast parallel processing\n",
    );
    help.push_str("  retrieve    Search symbols, find callers/callees, analyze impact\n");
    help.push_str("  config      Display active settings from .codanna/settings.toml\n");
    help.push_str("  mcp-test    Verify Claude can connect and list available tools\n");
    help.push_str("  mcp         Execute MCP tools without spawning server - for debugging\n");
    help.push_str("  benchmark   Benchmark parser performance for different languages\n");
    help.push_str("  help        Print this message or the help of the given subcommand(s)\n\n");

    // Options
    if Theme::should_disable_colors() {
        help.push_str("Options:\n");
    } else {
        help.push_str(&format!("{}\n", style("Options:").cyan().bold()));
    }
    help.push_str("  -c, --config <CONFIG>  Path to custom settings.toml file\n");
    help.push_str("      --info             Show detailed loading information\n");
    help.push_str("  -h, --help             Print help\n");
    help.push_str("  -V, --version          Print version\n\n");

    // Helper to format description lines with dimmed text
    let format_desc = |desc: &str| {
        if Theme::should_disable_colors() {
            format!("  {desc}:\n")
        } else {
            let styled = style(desc).dim();
            format!("  {styled}:\n")
        }
    };

    // Examples
    if Theme::should_disable_colors() {
        help.push_str("Examples:\n");
    } else {
        help.push_str(&format!("{}\n", style("Examples:").cyan().bold()));
    }

    help.push_str(&format_desc("First time setup"));
    help.push_str("    $ codanna init\n");
    help.push_str("    $ codanna index src --progress\n");
    help.push_str("    $ codanna mcp-test\n\n");

    help.push_str(&format_desc("Index a single file"));
    help.push_str("    $ codanna index src/main.rs\n\n");

    help.push_str(&format_desc("Check what calls your main function"));
    help.push_str("    $ codanna retrieve callers main\n\n");

    help.push_str(&format_desc("Natural language search"));
    help.push_str(
        "    $ codanna mcp semantic_search_docs --args '{\"query\": \"error handling\"}'\n\n",
    );

    help.push_str(&format_desc("Show detailed loading information"));
    help.push_str("    $ codanna --info retrieve symbol main\n\n");

    // Benchmarks
    if Theme::should_disable_colors() {
        help.push_str("Benchmarks:\n");
    } else {
        help.push_str(&format!("{}\n", style("Benchmarks:").cyan().bold()));
    }

    help.push_str(&format_desc("Test parser performance (all languages)"));
    help.push_str("    $ codanna benchmark all\n\n");

    help.push_str(&format_desc("Benchmark specific language"));
    help.push_str("    $ codanna benchmark python\n");
    help.push_str("    $ codanna benchmark rust\n\n");

    help.push_str(&format_desc("Benchmark with your own file"));
    help.push_str("    $ codanna benchmark python --file large_module.py\n\n");

    // Learn More
    if Theme::should_disable_colors() {
        help.push_str("Learn More:\n");
    } else {
        help.push_str(&format!("{}\n", style("Learn More:").cyan().bold()));
    }
    help.push_str("  GitHub: https://github.com/bartolli/codanna\n");
    help.push_str("  Commands: codanna help <COMMAND>");

    help
}

/// High-performance code intelligence for AI assistants
#[derive(Parser)]
#[command(
    name = "codanna",
    version = env!("CARGO_PKG_VERSION"),
    about = "High-performance code intelligence for AI assistants",
    long_about = "Codanna provides AI assistants like Claude with deep understanding of your codebase.\n\nFast parallel indexing with natural language search capabilities.",
    next_line_help = true,
    styles = clap_cargo_style(),
    override_help = create_custom_help()
)]
struct Cli {
    /// Path to custom settings.toml file
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Show detailed loading information
    #[arg(long, global = true)]
    info: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Available CLI commands
#[derive(Subcommand)]
enum Commands {
    /// Initialize project for code intelligence
    #[command(about = "Set up .codanna directory with default configuration")]
    Init {
        /// Force overwrite existing configuration
        #[arg(short, long)]
        force: bool,
    },

    /// Index source files or directories for AI understanding
    #[command(about = "Build searchable index from your codebase with fast parallel processing")]
    Index {
        /// Path to file or directory to index
        path: PathBuf,

        /// Number of threads to use (overrides config)
        #[arg(short, long)]
        threads: Option<usize>,

        /// Force re-indexing even if index exists
        #[arg(short, long)]
        force: bool,

        /// Show progress during indexing
        #[arg(short, long)]
        progress: bool,

        /// Dry run - show what would be indexed without indexing
        #[arg(long)]
        dry_run: bool,

        /// Maximum number of files to index
        #[arg(long)]
        max_files: Option<usize>,
    },

    /// Query code relationships and dependencies
    #[command(about = "Search symbols, find callers/callees, analyze impact")]
    Retrieve {
        #[command(subcommand)]
        query: RetrieveQuery,
    },

    /// Show current configuration settings
    #[command(about = "Display active settings from .codanna/settings.toml")]
    Config,

    /// Start MCP server for AI assistants
    #[command(hide = true)] // Hidden - used internally by MCP clients
    Serve {
        /// Watch index file for changes and auto-reload
        #[arg(long, help = "Enable hot-reload when index changes")]
        watch: bool,

        /// Check interval in seconds (default: 5)
        #[arg(
            long,
            default_value = "5",
            help = "How often to check for index changes"
        )]
        watch_interval: u64,

        /// Enable HTTP server mode instead of stdio
        #[arg(long, help = "Run as HTTP server instead of stdio transport")]
        http: bool,

        /// Enable HTTPS server mode with TLS
        #[arg(
            long,
            conflicts_with = "http",
            help = "Run as HTTPS server with TLS support"
        )]
        https: bool,

        /// Bind address for HTTP/HTTPS server
        #[arg(
            long,
            default_value = "127.0.0.1:8080",
            help = "Address to bind HTTP/HTTPS server to"
        )]
        bind: String,
    },

    /// Test MCP connection with Claude
    #[command(
        name = "mcp-test",
        about = "Verify Claude can connect and list available tools"
    )]
    McpTest {
        /// Path to server binary (defaults to current binary)
        #[arg(long)]
        server_binary: Option<PathBuf>,

        /// Tool to call (if not specified, just lists tools)
        #[arg(long)]
        tool: Option<String>,

        /// Tool arguments as JSON
        #[arg(long)]
        args: Option<String>,
    },

    /// Call MCP tools directly (advanced)
    #[command(about = "Execute MCP tools without spawning server - for debugging")]
    Mcp {
        /// Tool to call
        tool: String,

        /// Tool arguments as JSON
        #[arg(long)]
        args: Option<String>,
    },

    /// Run parser performance benchmarks
    #[command(about = "Benchmark parser performance for different languages")]
    Benchmark {
        /// Language to benchmark (rust, python, all)
        #[arg(default_value = "all")]
        language: String,

        /// Custom file to benchmark
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
}

/// Query types for retrieving indexed information.
///
/// Supports symbol lookups, relationship queries, impact analysis, and full-text search.
#[derive(Subcommand)]
enum RetrieveQuery {
    /// Find a symbol by name
    Symbol {
        /// Name of the symbol to find
        name: String,
    },

    /// Show what functions a given function calls
    Calls {
        /// Name of the function
        function: String,
    },

    /// Show what functions call a given function
    Callers {
        /// Name of the function
        function: String,
    },

    /// Show what types implement a given trait
    Implementations {
        /// Name of the trait
        trait_name: String,
    },

    /// Show what types a given symbol uses
    Uses {
        /// Name of the symbol
        symbol: String,
    },

    /// Show the impact radius of changing a symbol
    Impact {
        /// Name of the symbol
        symbol: String,
        /// Maximum depth to search (default: 5)
        #[arg(short, long)]
        depth: Option<usize>,
    },

    /// Search for symbols using full-text search
    Search {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Filter by symbol kind (e.g., Function, Struct, Trait)
        #[arg(short, long)]
        kind: Option<String>,

        /// Filter by module path
        #[arg(short, long)]
        module: Option<String>,
    },

    /// Show what methods a type or trait defines
    Defines {
        /// Name of the type or trait
        symbol: String,
    },

    /// Show comprehensive dependency analysis for a symbol
    Dependencies {
        /// Name of the symbol
        symbol: String,
    },

    /// Show comprehensive information about a symbol
    Describe {
        /// Name of the symbol
        symbol: String,
    },
}

/// Entry point with tokio async runtime.
///
/// Handles config initialization, index loading/creation, and command dispatch.
/// Auto-initializes config for index command. Persists index after modifications.
#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // For index command, auto-initialize if needed
    if matches!(cli.command, Commands::Index { .. }) {
        if Settings::check_init().is_err() {
            // Auto-initialize for index command
            eprintln!("Initializing project configuration...");
            match Settings::init_config_file(false) {
                Ok(path) => {
                    eprintln!("Created configuration file at: {}", path.display());
                }
                Err(e) => {
                    eprintln!("Warning: Could not create config file: {e}");
                    eprintln!("Using default configuration.");
                }
            }
        }
    } else if !matches!(cli.command, Commands::Init { .. }) {
        // For other commands, just warn
        if let Err(warning) = Settings::check_init() {
            eprintln!("Warning: {warning}");
            eprintln!("Using default configuration for now.");
        }
    }

    // Load configuration
    let mut config = if let Some(config_path) = &cli.config {
        Settings::load_from(config_path).unwrap_or_else(|e| {
            eprintln!(
                "Configuration error loading from {}: {}",
                config_path.display(),
                e
            );
            std::process::exit(1);
        })
    } else {
        Settings::load().unwrap_or_else(|e| {
            eprintln!("Configuration error: {e}");
            Settings::default()
        })
    };

    match &cli.command {
        Commands::Init { force } => {
            let config_path = PathBuf::from(".codanna/settings.toml");

            if config_path.exists() && !force {
                eprintln!(
                    "Configuration file already exists at: {}",
                    config_path.display()
                );
                eprintln!("Use --force to overwrite");
                std::process::exit(1);
            }

            match Settings::init_config_file(*force) {
                Ok(path) => {
                    println!("Created configuration file at: {}", path.display());
                    println!("Edit this file to customize your settings.");
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
            return;
        }

        Commands::Config => {
            println!("Current Configuration:");
            println!("{}", "=".repeat(50));
            match toml::to_string_pretty(&config) {
                Ok(toml_str) => println!("{toml_str}"),
                Err(e) => eprintln!("Error displaying config: {e}"),
            }
            return;
        }

        Commands::Index {
            threads: Some(t),
            force: _,
            ..
        } => {
            // Override config with CLI args
            config.indexing.parallel_threads = *t;
        }
        Commands::Index {
            threads: None,
            force: _,
            ..
        } => {
            // Use default from config
        }

        Commands::Serve { .. } => {
            // No configuration overrides for serve
        }

        _ => {}
    }

    // Set up persistence based on config
    let index_path = config.index_path.clone();
    let persistence = IndexPersistence::new(index_path);

    // Skip loading index for mcp-test (thin client mode)
    let skip_index_load = matches!(cli.command, Commands::McpTest { .. });

    // Load existing index or create new one (unless we're in thin client mode)
    let settings = Arc::new(config.clone());
    let mut indexer = if skip_index_load {
        SimpleIndexer::with_settings(settings.clone()) // Empty indexer, won't be used
    } else {
        let force_recreate_index =
            matches!(cli.command, Commands::Index { force: true, ref path, .. } if path.is_dir());
        if persistence.exists() && !force_recreate_index {
            if config.debug {
                eprintln!(
                    "DEBUG: Found existing index at {}",
                    config.index_path.display()
                );
            }
            match persistence.load_with_settings(settings.clone(), cli.info) {
                Ok(loaded) => {
                    if config.debug {
                        eprintln!("DEBUG: Successfully loaded index from disk");
                    }
                    if cli.info {
                        eprintln!(
                            "Loaded existing index (total: {} symbols)",
                            loaded.symbol_count()
                        );
                    }
                    loaded
                }
                Err(e) => {
                    eprintln!("Warning: Could not load index: {e}. Creating new index.");
                    SimpleIndexer::with_settings(settings.clone())
                }
            }
        } else {
            if force_recreate_index && persistence.exists() {
                eprintln!("Force re-indexing requested, creating new index");
            } else if !persistence.exists() && config.debug {
                eprintln!(
                    "DEBUG: No existing index found at {}",
                    config.index_path.display()
                );
            }
            if config.debug {
                eprintln!("DEBUG: Creating new index");
            }
            let mut new_indexer = SimpleIndexer::with_settings(settings.clone());
            // Clear Tantivy index if force re-indexing directory
            if force_recreate_index {
                if let Err(e) = new_indexer.clear_tantivy_index() {
                    eprintln!("Warning: Failed to clear Tantivy index: {e}");
                }
            }
            new_indexer
        }
    };

    // Enable semantic search if configured
    if config.semantic_search.enabled && !indexer.has_semantic_search() {
        if let Err(e) = indexer.enable_semantic_search() {
            eprintln!("Warning: Failed to enable semantic search: {e}");
        } else {
            eprintln!(
                "Semantic search enabled (model: {}, threshold: {})",
                config.semantic_search.model, config.semantic_search.threshold
            );
        }
    }

    match cli.command {
        Commands::Init { .. } | Commands::Config => {
            // Already handled above
            unreachable!()
        }

        Commands::Serve {
            watch,
            watch_interval,
            http,
            https,
            bind,
        } => {
            // Determine server mode:
            // 1. CLI --https flag takes highest precedence
            // 2. CLI --http flag takes second precedence
            // 3. Otherwise, check config.server.mode
            let server_mode = if https {
                "https"
            } else if http || config.server.mode == "http" {
                "http"
            } else {
                "stdio"
            };

            // Use bind address from CLI if provided, otherwise from config
            // For HTTPS, default to port 8443 if using default bind
            let bind_address = if bind != "127.0.0.1:8080" {
                // CLI flag was explicitly set (not default)
                bind
            } else if https {
                // For HTTPS, use port 8443 by default
                "127.0.0.1:8443".to_string()
            } else {
                // Use config value
                config.server.bind.clone()
            };

            // Use watch interval from CLI if provided, otherwise from config
            let actual_watch_interval = if watch_interval != 5 {
                // CLI flag was explicitly set (not default)
                watch_interval
            } else {
                config.server.watch_interval
            };

            match server_mode {
                "https" => {
                    // HTTPS mode - secure server with TLS
                    if config.mcp.debug {
                        eprintln!("Starting MCP server in HTTPS mode with TLS");
                        eprintln!("Bind address: {bind_address}");
                        if watch || config.file_watch.enabled {
                            eprintln!(
                                "File watching: ENABLED (event-driven with {}ms debounce)",
                                config.file_watch.debounce_ms
                            );
                        }
                    }

                    // Use the HTTPS server implementation
                    #[cfg(feature = "https-server")]
                    {
                        use codanna::mcp::https_server::serve_https;
                        if let Err(e) = serve_https(config, watch, bind_address).await {
                            eprintln!("HTTPS server error: {e}");
                            std::process::exit(1);
                        }
                    }

                    #[cfg(not(feature = "https-server"))]
                    {
                        eprintln!("HTTPS server support is not compiled in.");
                        eprintln!("Please rebuild with: cargo build --features https-server");
                        std::process::exit(1);
                    }
                }
                "http" => {
                    // HTTP mode - persistent server with event-driven file watching
                    eprintln!("Starting MCP server in HTTP mode");
                    eprintln!("Bind address: {bind_address}");
                    if watch || config.file_watch.enabled {
                        eprintln!(
                            "File watching: ENABLED (event-driven with {}ms debounce)",
                            config.file_watch.debounce_ms
                        );
                    }

                    // Use the HTTP server implementation
                    use codanna::mcp::http_server::serve_http;
                    if let Err(e) = serve_http(config, watch, bind_address).await {
                        eprintln!("HTTP server error: {e}");
                        std::process::exit(1);
                    }
                }
                _ => {
                    // stdio mode - current implementation
                    eprintln!("Starting MCP server on stdio transport");
                    if watch {
                        eprintln!("Index watching enabled (interval: {actual_watch_interval}s)");
                    }
                    eprintln!("To test: npx @modelcontextprotocol/inspector cargo run -- serve");

                    // Create MCP server from existing index
                    let server = codanna::mcp::CodeIntelligenceServer::from_persistence(&config)
                        .await
                        .map_err(|e| {
                            eprintln!("Failed to create MCP server: {e}");
                            std::process::exit(1);
                        })
                        .unwrap();

                    // If watch mode is enabled, start the index watcher
                    if watch {
                        use codanna::mcp::watcher::IndexWatcher;
                        use std::time::Duration;

                        let indexer_arc = server.get_indexer_arc();
                        let settings = Arc::new(config.clone());
                        let watcher = IndexWatcher::new(
                            indexer_arc,
                            settings,
                            Duration::from_secs(actual_watch_interval),
                        );

                        // Spawn watcher in background
                        tokio::spawn(async move {
                            watcher.watch().await;
                        });

                        eprintln!("Index watcher started");
                    }

                    // If file watching is enabled in config, start the file system watcher
                    if config.file_watch.enabled {
                        use codanna::indexing::FileSystemWatcher;

                        eprintln!("Starting file system watcher for indexed files");
                        eprintln!("  Debounce interval: {}ms", config.file_watch.debounce_ms);

                        let watcher_indexer = server.get_indexer_arc();
                        let watcher = FileSystemWatcher::new(
                            watcher_indexer,
                            config.file_watch.debounce_ms,
                            config.mcp.debug,
                        )
                        .map_err(|e| {
                            eprintln!("Failed to create file system watcher: {e}");
                            eprintln!("File watching disabled for this session");
                            e
                        });

                        if let Ok(watcher) = watcher {
                            // Spawn file watcher in background
                            tokio::spawn(async move {
                                if let Err(e) = watcher.watch().await {
                                    eprintln!("File watcher error: {e}");
                                }
                            });
                            eprintln!(
                                "File system watcher started - monitoring indexed files for changes"
                            );
                        }
                    }

                    // Start server with stdio transport
                    use rmcp::{ServiceExt, transport::stdio};
                    let service = server
                        .serve(stdio())
                        .await
                        .map_err(|e| {
                            eprintln!("Failed to start MCP server: {e}");
                            std::process::exit(1);
                        })
                        .unwrap();

                    // Wait for server to complete
                    service
                        .waiting()
                        .await
                        .map_err(|e| {
                            eprintln!("MCP server error: {e}");
                            std::process::exit(1);
                        })
                        .unwrap();
                } // End of else block for stdio mode
            } // End of match
        }

        Commands::Index {
            path,
            force,
            progress,
            dry_run,
            max_files,
            ..
        } => {
            // Determine if path is a file or directory
            if path.is_file() {
                // Single file indexing
                match indexer.index_file_with_force(&path, force) {
                    Ok(result) => {
                        let language_name = codanna::parsing::Language::from_path(&path)
                            .map(|l| l.to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                        if result.is_cached() {
                            println!(
                                "Successfully loaded from cache: {} [{}]",
                                path.display(),
                                language_name
                            );
                        } else {
                            println!(
                                "Successfully indexed: {} [{}]",
                                path.display(),
                                language_name
                            );
                        }
                        println!("File ID: {}", result.file_id().value());

                        // Get symbols for just this file
                        let file_symbols = indexer.get_symbols_by_file(result.file_id());
                        println!("Found {} symbols in this file", file_symbols.len());
                        println!("Total symbols in index: {}", indexer.symbol_count());

                        // Show summary of what was found in this file
                        let functions = file_symbols
                            .iter()
                            .filter(|s| s.kind == SymbolKind::Function)
                            .count();
                        let methods = file_symbols
                            .iter()
                            .filter(|s| s.kind == SymbolKind::Method)
                            .count();
                        let structs = file_symbols
                            .iter()
                            .filter(|s| s.kind == SymbolKind::Struct)
                            .count();
                        let traits = file_symbols
                            .iter()
                            .filter(|s| s.kind == SymbolKind::Trait)
                            .count();

                        println!("  Functions: {functions}");
                        println!("  Methods: {methods}");
                        println!("  Structs: {structs}");
                        println!("  Traits: {traits}");

                        // Save the index
                        if config.debug {
                            eprintln!(
                                "DEBUG: Saving index with {} symbols",
                                indexer.symbol_count()
                            );
                        }
                        match persistence.save(&indexer) {
                            Ok(_) => {
                                println!("\nIndex saved to: {}", config.index_path.display());
                                if config.debug {
                                    eprintln!("DEBUG: Index saved successfully");
                                }
                            }
                            Err(e) => eprintln!("\nWarning: Could not save index: {e}"),
                        }
                    }
                    Err(e) => {
                        eprintln!("Error indexing file: {e}");

                        // Display recovery suggestions
                        let suggestions = e.recovery_suggestions();
                        if !suggestions.is_empty() {
                            eprintln!("\nSuggestions:");
                            for suggestion in suggestions {
                                eprintln!("  • {suggestion}");
                            }
                        }

                        std::process::exit(1);
                    }
                }
            } else if path.is_dir() {
                // Directory indexing
                if let Some(max) = max_files {
                    println!(
                        "Indexing directory: {} (limited to {} files)",
                        path.display(),
                        max
                    );
                } else {
                    println!("Indexing directory: {}", path.display());
                }

                match indexer
                    .index_directory_with_options(&path, progress, dry_run, force, max_files)
                {
                    Ok(stats) => {
                        stats.display();

                        if !dry_run && stats.files_indexed > 0 {
                            // Save the index
                            eprintln!(
                                "\nSaving index with {} total symbols, {} total relationships...",
                                indexer.symbol_count(),
                                indexer.relationship_count()
                            );
                            match persistence.save(&indexer) {
                                Ok(_) => {
                                    println!("Index saved to: {}", config.index_path.display());
                                }
                                Err(e) => {
                                    eprintln!("Error: Could not save index: {e}");
                                    std::process::exit(1);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error indexing directory: {e}");

                        // Display recovery suggestions
                        let suggestions = e.recovery_suggestions();
                        if !suggestions.is_empty() {
                            eprintln!("\nSuggestions:");
                            for suggestion in suggestions {
                                eprintln!("  • {suggestion}");
                            }
                        }

                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("Error: Path does not exist: {}", path.display());
                std::process::exit(1);
            }
        }

        Commands::Retrieve { query } => {
            match query {
                RetrieveQuery::Symbol { name } => {
                    let symbols = indexer.find_symbols_by_name(&name);

                    if symbols.is_empty() {
                        println!("No symbols found with name: {name}");
                    } else {
                        println!("Found {} symbol(s) named '{}':", symbols.len(), name);
                        for symbol in symbols {
                            let file_path = indexer
                                .get_file_path(symbol.file_id)
                                .unwrap_or_else(|| "<unknown>".to_string());
                            println!(
                                "  {:?} at {}:{}",
                                symbol.kind,
                                file_path,
                                symbol.range.start_line + 1
                            );

                            // Show documentation if available
                            if let Some(ref doc) = symbol.doc_comment {
                                // Show first 3 lines or less
                                let lines: Vec<&str> = doc.lines().take(3).collect();
                                let preview = if doc.lines().count() > 3 {
                                    format!("{}...", lines.join(" "))
                                } else {
                                    lines.join(" ")
                                };
                                println!("    Documentation: {preview}");
                            }

                            // Show signature if available
                            if let Some(ref sig) = symbol.signature {
                                println!("    Signature: {sig}");
                            }
                        }
                    }
                }

                RetrieveQuery::Calls { function } => {
                    let symbols = indexer.find_symbols_by_name(&function);

                    if symbols.is_empty() {
                        println!("Function not found: {function}");
                    } else {
                        let mut all_called_with_metadata = Vec::new();
                        let mut checked_symbols = 0;

                        // Check all symbols with this name
                        for symbol in &symbols {
                            checked_symbols += 1;
                            let called = indexer.get_called_functions_with_metadata(symbol.id);
                            for (callee, metadata) in called {
                                if !all_called_with_metadata
                                    .iter()
                                    .any(|(c, _): &(Symbol, _)| c.id == callee.id)
                                {
                                    all_called_with_metadata.push((callee, metadata));
                                }
                            }
                        }

                        if all_called_with_metadata.is_empty() {
                            println!(
                                "{function} doesn't call any functions (checked {checked_symbols} symbol(s) with this name)"
                            );
                        } else {
                            println!(
                                "{} calls {} function(s):",
                                function,
                                all_called_with_metadata.len()
                            );
                            for (callee, metadata) in all_called_with_metadata {
                                // Parse metadata to extract receiver info
                                let call_display = if let Some(meta) = metadata {
                                    if meta.contains("receiver:") && meta.contains("static:") {
                                        // Parse "receiver:{receiver},static:{is_static}"
                                        let parts: Vec<&str> = meta.split(',').collect();
                                        let mut receiver = "";
                                        let mut is_static = false;

                                        for part in parts {
                                            if let Some(r) = part.strip_prefix("receiver:") {
                                                receiver = r;
                                            } else if let Some(s) = part.strip_prefix("static:") {
                                                is_static = s == "true";
                                            }
                                        }

                                        if !receiver.is_empty() {
                                            if is_static {
                                                format!("{}::{}", receiver, callee.name)
                                            } else {
                                                format!("{}.{}", receiver, callee.name)
                                            }
                                        } else {
                                            callee.name.to_string()
                                        }
                                    } else {
                                        callee.name.to_string()
                                    }
                                } else {
                                    callee.name.to_string()
                                };
                                println!("  -> {call_display}");
                            }
                        }
                    }
                }

                RetrieveQuery::Callers { function } => {
                    let symbols = indexer.find_symbols_by_name(&function);

                    if symbols.is_empty() {
                        println!("Function not found: {function}");
                    } else {
                        let mut all_callers_with_metadata = Vec::new();
                        let mut checked_symbols = 0;

                        // Check all symbols with this name
                        for symbol in &symbols {
                            checked_symbols += 1;
                            let callers = indexer.get_calling_functions_with_metadata(symbol.id);
                            for (caller, metadata) in callers {
                                if !all_callers_with_metadata
                                    .iter()
                                    .any(|(c, _): &(Symbol, _)| c.id == caller.id)
                                {
                                    all_callers_with_metadata.push((caller, metadata));
                                }
                            }
                        }

                        if all_callers_with_metadata.is_empty() {
                            println!(
                                "No functions call {function} (checked {checked_symbols} symbol(s) with this name)"
                            );
                        } else {
                            println!(
                                "{} function(s) call {}:",
                                all_callers_with_metadata.len(),
                                function
                            );
                            for (caller, metadata) in all_callers_with_metadata {
                                let file_path = indexer
                                    .get_file_path(caller.file_id)
                                    .unwrap_or_else(|| "<unknown>".to_string());

                                // Parse metadata to extract receiver info
                                let call_display = if let Some(meta) = metadata {
                                    if meta.contains("receiver:") && meta.contains("static:") {
                                        // Parse "receiver:{receiver},static:{is_static}"
                                        let parts: Vec<&str> = meta.split(',').collect();
                                        let mut receiver = "";
                                        let mut is_static = false;

                                        for part in parts {
                                            if let Some(r) = part.strip_prefix("receiver:") {
                                                receiver = r;
                                            } else if let Some(s) = part.strip_prefix("static:") {
                                                is_static = s == "true";
                                            }
                                        }

                                        let call_str = if !receiver.is_empty() {
                                            if is_static {
                                                format!("{receiver}::{function}")
                                            } else {
                                                format!("{receiver}.{function}")
                                            }
                                        } else {
                                            function.to_string()
                                        };

                                        format!("{} calls {}", caller.name, call_str)
                                    } else {
                                        caller.name.to_string()
                                    }
                                } else {
                                    caller.name.to_string()
                                };

                                println!(
                                    "  <- {} ({}:{})",
                                    call_display,
                                    file_path,
                                    caller.range.start_line + 1
                                );
                            }
                        }
                    }
                }

                RetrieveQuery::Implementations { trait_name } => {
                    use codanna::symbol::context::ContextIncludes;

                    // Find all symbols with this name and look for the trait
                    let symbols = indexer.find_symbols_by_name(&trait_name);
                    let trait_symbol = symbols.iter().find(|s| s.kind == SymbolKind::Trait);

                    match trait_symbol {
                        Some(symbol) => {
                            let ctx = indexer.get_symbol_context(
                                symbol.id,
                                ContextIncludes::IMPLEMENTATIONS | ContextIncludes::DEFINITIONS,
                            );

                            if let Some(ctx) = ctx {
                                if let Some(impls) = &ctx.relationships.implemented_by {
                                    if impls.is_empty() {
                                        println!("No types implement {trait_name}");
                                    } else {
                                        println!(
                                            "{} type(s) implement {}:",
                                            impls.len(),
                                            trait_name
                                        );

                                        // Show trait methods first
                                        if let Some(defines) = &ctx.relationships.defines {
                                            let methods: Vec<_> = defines
                                                .iter()
                                                .filter(|s| s.kind == SymbolKind::Method)
                                                .map(|s| s.as_name())
                                                .collect();
                                            if !methods.is_empty() {
                                                println!("Trait methods: {}", methods.join(", "));
                                                println!();
                                            }
                                        }

                                        // Show each implementation with context
                                        for impl_type in impls {
                                            let impl_ctx = indexer.get_symbol_context(
                                                impl_type.id,
                                                ContextIncludes::DEFINITIONS,
                                            );

                                            println!("  - {}", impl_type.name);
                                            println!("    Type: {:?}", impl_type.kind);

                                            if let Some(impl_ctx) = impl_ctx {
                                                println!(
                                                    "    Location: {}:{}",
                                                    impl_ctx.file_path,
                                                    impl_type.range.start_line + 1
                                                );

                                                if let Some(module) = impl_type.as_module_path() {
                                                    println!("    Module: {module}");
                                                }

                                                // Check for test annotation
                                                if impl_type.name.contains("Mock")
                                                    || impl_type.name.contains("Test")
                                                {
                                                    println!(
                                                        "    Note: Likely test implementation"
                                                    );
                                                }
                                            } else {
                                                // Fallback if we can't get context
                                                let file_path = indexer
                                                    .get_file_path(impl_type.file_id)
                                                    .unwrap_or_else(|| "<unknown>".to_string());
                                                println!(
                                                    "    Location: {}:{}",
                                                    file_path,
                                                    impl_type.range.start_line + 1
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    println!("No types implement {trait_name}");
                                }
                            } else {
                                // Fallback to original behavior if context fails
                                let implementations = indexer.get_implementations(symbol.id);
                                if implementations.is_empty() {
                                    println!("No types implement {trait_name}");
                                } else {
                                    println!(
                                        "{} type(s) implement {}:",
                                        implementations.len(),
                                        trait_name
                                    );
                                    for impl_type in implementations {
                                        println!("  - {}", impl_type.name);
                                    }
                                }
                            }
                        }
                        None => {
                            println!("Trait not found: {trait_name}");
                        }
                    }
                }

                RetrieveQuery::Uses { symbol } => match indexer.find_symbol(&symbol) {
                    Some(symbol_id) => {
                        let dependencies = indexer.get_dependencies(symbol_id);
                        let used_types = dependencies
                            .get(&RelationKind::Uses)
                            .cloned()
                            .unwrap_or_default();

                        if used_types.is_empty() {
                            println!("{symbol} doesn't use any types");
                        } else {
                            println!("{} uses {} type(s):", symbol, used_types.len());
                            for used in used_types {
                                println!("  - {}", used.name);
                            }
                        }
                    }
                    None => {
                        println!("Symbol not found: {symbol}");
                    }
                },

                RetrieveQuery::Impact { symbol, depth } => {
                    match indexer.find_symbol(&symbol) {
                        Some(symbol_id) => {
                            let impacted = indexer.get_impact_radius(symbol_id, depth);

                            if impacted.is_empty() {
                                println!("No symbols would be impacted by changing {symbol}");
                            } else {
                                println!(
                                    "Changing {} would impact {} symbol(s):",
                                    symbol,
                                    impacted.len()
                                );

                                // Group by symbol kind for better readability
                                let mut by_kind: std::collections::HashMap<SymbolKind, Vec<_>> =
                                    std::collections::HashMap::new();
                                for id in impacted {
                                    if let Some(sym) = indexer.get_symbol(id) {
                                        by_kind.entry(sym.kind).or_default().push(sym);
                                    }
                                }

                                // Display grouped by kind
                                for (kind, symbols) in by_kind {
                                    println!("\n  {}s:", format!("{kind:?}").to_lowercase());
                                    for sym in symbols {
                                        let file_path = indexer
                                            .get_file_path(sym.file_id)
                                            .unwrap_or_else(|| "<unknown>".to_string());
                                        println!(
                                            "    - {} ({}:{})",
                                            sym.name,
                                            file_path,
                                            sym.range.start_line + 1
                                        );
                                    }
                                }
                            }
                        }
                        None => {
                            println!("Symbol not found: {symbol}");
                        }
                    }
                }

                RetrieveQuery::Search {
                    query,
                    limit,
                    kind,
                    module,
                } => {
                    // Parse the kind filter if provided
                    let kind_filter = kind.as_ref().and_then(|k| match k.to_lowercase().as_str() {
                        "function" => Some(SymbolKind::Function),
                        "struct" => Some(SymbolKind::Struct),
                        "trait" => Some(SymbolKind::Trait),
                        "method" => Some(SymbolKind::Method),
                        "field" => Some(SymbolKind::Field),
                        "module" => Some(SymbolKind::Module),
                        "constant" => Some(SymbolKind::Constant),
                        _ => {
                            eprintln!("Warning: Unknown symbol kind '{k}', ignoring filter");
                            None
                        }
                    });

                    match indexer.search(&query, limit, kind_filter, module.as_deref()) {
                        Ok(results) => {
                            if results.is_empty() {
                                println!("No results found for query: {query}");
                            } else {
                                println!(
                                    "Found {} result(s) for query '{}':\n",
                                    results.len(),
                                    query
                                );

                                for (i, result) in results.iter().enumerate() {
                                    println!("{}. {} ({:?})", i + 1, result.name, result.kind);
                                    println!("   File: {}:{}", result.file_path, result.line);
                                    if !result.module_path.is_empty() {
                                        println!("   Module: {}", result.module_path);
                                    }
                                    if let Some(ref doc) = result.doc_comment {
                                        println!("   Doc: {}", doc.lines().next().unwrap_or(""));
                                    }
                                    println!("   Score: {:.2}", result.score);
                                    println!();
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Search failed: {e}");
                        }
                    }
                }

                RetrieveQuery::Defines { symbol } => match indexer.find_symbol(&symbol) {
                    Some(symbol_id) => {
                        use codanna::symbol::context::ContextIncludes;

                        let ctx =
                            indexer.get_symbol_context(symbol_id, ContextIncludes::DEFINITIONS);

                        if let Some(ctx) = ctx {
                            if let Some(defines) = &ctx.relationships.defines {
                                if defines.is_empty() {
                                    println!("{symbol} doesn't define any symbols");
                                } else {
                                    // Group by kind
                                    let methods: Vec<_> = defines
                                        .iter()
                                        .filter(|s| s.kind == SymbolKind::Method)
                                        .collect();
                                    let fields: Vec<_> = defines
                                        .iter()
                                        .filter(|s| s.kind == SymbolKind::Field)
                                        .collect();
                                    let others: Vec<_> = defines
                                        .iter()
                                        .filter(|s| {
                                            !matches!(
                                                s.kind,
                                                SymbolKind::Method | SymbolKind::Field
                                            )
                                        })
                                        .collect();

                                    println!(
                                        "{} ({:?}) defines {} symbol(s):",
                                        ctx.symbol.name,
                                        ctx.symbol.kind,
                                        defines.len()
                                    );
                                    println!("Location: {}", ctx.format_location());

                                    if !methods.is_empty() {
                                        println!("\nMethods ({}):", methods.len());
                                        for method in methods {
                                            print!("  - {}", method.name);
                                            if let Some(sig) = method.as_signature() {
                                                println!(" :: {sig}");
                                            } else {
                                                println!();
                                            }
                                        }
                                    }

                                    if !fields.is_empty() {
                                        println!("\nFields ({}):", fields.len());
                                        for field in fields {
                                            println!("  - {}", field.name);
                                        }
                                    }

                                    if !others.is_empty() {
                                        println!("\nOther ({}):", others.len());
                                        for other in others {
                                            println!("  - {} ({:?})", other.name, other.kind);
                                        }
                                    }
                                }
                            } else {
                                println!("{symbol} doesn't define any symbols");
                            }
                        } else {
                            // Fallback
                            let dependencies = indexer.get_dependencies(symbol_id);
                            let defined = dependencies
                                .get(&RelationKind::Defines)
                                .cloned()
                                .unwrap_or_default();

                            if defined.is_empty() {
                                println!("{symbol} doesn't define any methods");
                            } else {
                                println!("{} defines {} method(s):", symbol, defined.len());
                                for def in defined {
                                    println!("  - {}", def.name);
                                }
                            }
                        }
                    }
                    None => {
                        println!("Symbol not found: {symbol}");
                    }
                },

                RetrieveQuery::Dependencies { symbol } => {
                    match indexer.find_symbol(&symbol) {
                        Some(symbol_id) => {
                            use codanna::symbol::context::ContextIncludes;

                            let ctx = indexer.get_symbol_context(symbol_id, ContextIncludes::ALL);

                            if let Some(ctx) = ctx {
                                println!(
                                    "Dependency Analysis for {} ({:?}):",
                                    ctx.symbol.name, ctx.symbol.kind
                                );
                                println!("Location: {}", ctx.format_location());
                                println!("{}", "=".repeat(60));

                                // Show what this symbol defines
                                if let Some(defines) = &ctx.relationships.defines {
                                    if !defines.is_empty() {
                                        println!("\nDefines ({}):", defines.len());
                                        for def in defines {
                                            print!("  → {} ({:?})", def.name, def.kind);
                                            if let Some(sig) = def.as_signature() {
                                                print!(" :: {sig}");
                                            }
                                            println!();
                                        }
                                    }
                                }

                                // Show what this symbol calls (with metadata)
                                if let Some(calls) = &ctx.relationships.calls {
                                    if !calls.is_empty() {
                                        println!("\nCalls ({}):", calls.len());
                                        for (called, metadata) in calls {
                                            print!("  → {} ({:?})", called.name, called.kind);
                                            if let Some(meta) = metadata {
                                                // Parse receiver metadata
                                                if meta.contains("receiver:")
                                                    && meta.contains("static:")
                                                {
                                                    let parts: Vec<&str> =
                                                        meta.split(',').collect();
                                                    if parts.len() == 2 {
                                                        let receiver = parts[0]
                                                            .trim_start_matches("receiver:");
                                                        let is_static = parts[1]
                                                            .trim_start_matches("static:")
                                                            == "true";

                                                        if is_static {
                                                            print!(" [static call]");
                                                        } else if !receiver.is_empty()
                                                            && receiver != "self"
                                                        {
                                                            print!(" [via {receiver}]");
                                                        }
                                                    }
                                                }
                                            }
                                            println!();
                                        }
                                    }
                                }

                                // Show who calls this symbol (with metadata)
                                if let Some(callers) = &ctx.relationships.called_by {
                                    if !callers.is_empty() {
                                        println!("\nCalled by ({}):", callers.len());
                                        for (caller, metadata) in callers {
                                            print!("  ← {} ({:?})", caller.name, caller.kind);
                                            if let Some(meta) = metadata {
                                                // Parse receiver metadata for context
                                                if meta.contains("receiver:")
                                                    && meta.contains("static:")
                                                {
                                                    let parts: Vec<&str> =
                                                        meta.split(',').collect();
                                                    if parts.len() == 2 {
                                                        let is_static = parts[1]
                                                            .trim_start_matches("static:")
                                                            == "true";
                                                        if is_static {
                                                            print!(" [as static method]");
                                                        }
                                                    }
                                                }
                                            }
                                            println!();
                                        }
                                    }
                                }

                                // Show implementations
                                if ctx.symbol.kind == SymbolKind::Trait {
                                    if let Some(impls) = &ctx.relationships.implemented_by {
                                        if !impls.is_empty() {
                                            println!("\nImplemented by ({}):", impls.len());
                                            for impl_type in impls {
                                                println!(
                                                    "  ← {} ({:?})",
                                                    impl_type.name, impl_type.kind
                                                );
                                            }
                                        }
                                    }
                                } else if let Some(impls) = &ctx.relationships.implements {
                                    if !impls.is_empty() {
                                        println!("\nImplements ({}):", impls.len());
                                        for trait_type in impls {
                                            println!("  → {} (Trait)", trait_type.name);
                                        }
                                    }
                                }

                                // Additional outgoing dependencies
                                let dependencies = indexer.get_dependencies(symbol_id);
                                let other_deps: Vec<_> = dependencies
                                    .iter()
                                    .filter(|(k, _)| {
                                        !matches!(
                                            k,
                                            RelationKind::Calls
                                                | RelationKind::Defines
                                                | RelationKind::Implements
                                        )
                                    })
                                    .collect();

                                if !other_deps.is_empty() {
                                    println!("\nOther Dependencies:");
                                    for (kind, symbols) in other_deps {
                                        if !symbols.is_empty() {
                                            println!("\n  {kind:?}:");
                                            for sym in symbols {
                                                println!("    → {} ({:?})", sym.name, sym.kind);
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Fallback to original behavior
                                let sym = indexer.get_symbol(symbol_id).unwrap();
                                println!("Dependency Analysis for {} ({:?}):", symbol, sym.kind);
                                println!("{}", "=".repeat(50));

                                let dependencies = indexer.get_dependencies(symbol_id);
                                if dependencies.is_empty() {
                                    println!("\nNo outgoing dependencies");
                                } else {
                                    println!("\nOutgoing Dependencies (what {symbol} depends on):");
                                    for (kind, symbols) in dependencies {
                                        if !symbols.is_empty() {
                                            println!("\n  {kind:?}:");
                                            for sym in symbols {
                                                println!("    → {} ({:?})", sym.name, sym.kind);
                                            }
                                        }
                                    }
                                }

                                let dependents = indexer.get_dependents(symbol_id);
                                if dependents.is_empty() {
                                    println!("\nNo incoming dependencies");
                                } else {
                                    println!("\nIncoming Dependencies (what depends on {symbol}):");
                                    for (kind, symbols) in dependents {
                                        if !symbols.is_empty() {
                                            println!("\n  {kind:?} by:");
                                            for sym in symbols {
                                                println!("    ← {} ({:?})", sym.name, sym.kind);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            println!("Symbol not found: {symbol}");
                        }
                    }
                }

                RetrieveQuery::Describe { symbol } => {
                    match indexer.find_symbol(&symbol) {
                        Some(symbol_id) => {
                            use codanna::symbol::context::ContextIncludes;

                            let ctx = indexer.get_symbol_context(symbol_id, ContextIncludes::ALL);

                            if let Some(ctx) = ctx {
                                // Use the format_full method for comprehensive output
                                println!("{}", ctx.format_full(""));

                                // Add additional context about relationships
                                if let Some(calls) = &ctx.relationships.calls {
                                    if !calls.is_empty() {
                                        println!("\nCall Details:");
                                        for (called, metadata) in calls.iter().take(5) {
                                            print!("  → {} ", called.name);
                                            if let Some(meta) = metadata {
                                                if meta.contains("receiver:")
                                                    && meta.contains("static:")
                                                {
                                                    let parts: Vec<&str> =
                                                        meta.split(',').collect();
                                                    if parts.len() == 2 {
                                                        let receiver = parts[0]
                                                            .trim_start_matches("receiver:");
                                                        let is_static = parts[1]
                                                            .trim_start_matches("static:")
                                                            == "true";

                                                        if is_static {
                                                            print!("(static call)");
                                                        } else if !receiver.is_empty()
                                                            && receiver != "self"
                                                        {
                                                            print!("(via {receiver})");
                                                        } else {
                                                            print!("(method call)");
                                                        }
                                                    }
                                                }
                                            }
                                            println!();
                                        }
                                        if calls.len() > 5 {
                                            println!("  ... and {} more", calls.len() - 5);
                                        }
                                    }
                                }

                                if let Some(callers) = &ctx.relationships.called_by {
                                    if !callers.is_empty() {
                                        println!("\nCaller Details:");
                                        for (caller, metadata) in callers.iter().take(5) {
                                            print!("  ← {} ", caller.name);
                                            if let Some(meta) = metadata {
                                                if meta.contains("static:true") {
                                                    print!("(as static method)");
                                                } else {
                                                    print!("(as instance method)");
                                                }
                                            }
                                            println!();
                                        }
                                        if callers.len() > 5 {
                                            println!("  ... and {} more", callers.len() - 5);
                                        }
                                    }
                                }
                            } else {
                                // Fallback: just show basic symbol info
                                if let Some(sym) = indexer.get_symbol(symbol_id) {
                                    println!("{} ({:?})", sym.name, sym.kind);
                                    if let Some(path) = indexer.get_file_path(sym.file_id) {
                                        println!("Location: {}:{}", path, sym.range.start_line + 1);
                                    }
                                    if let Some(sig) = sym.as_signature() {
                                        println!("Signature: {sig}");
                                    }
                                    if let Some(doc) = sym.as_doc_comment() {
                                        println!("Documentation:\n{doc}");
                                    }
                                }
                            }
                        }
                        None => {
                            println!("Symbol not found: {symbol}");
                        }
                    }
                }
            }
        }

        Commands::McpTest {
            server_binary,
            tool: _,
            args: _,
        } => {
            use codanna::mcp::client::CodeIntelligenceClient;

            // Get server binary path (default to current executable)
            let server_path = server_binary.unwrap_or_else(|| {
                std::env::current_exe().expect("Failed to get current executable path")
            });

            // Run the test
            if let Err(e) = CodeIntelligenceClient::test_server(server_path).await {
                eprintln!("MCP test failed: {e}");
                std::process::exit(1);
            }
        }

        Commands::Mcp { tool, args } => {
            // Embedded mode - use already loaded indexer directly
            let server = codanna::mcp::CodeIntelligenceServer::new(indexer);

            // Parse arguments if provided
            let arguments = if let Some(args_str) = args {
                match serde_json::from_str::<serde_json::Value>(&args_str) {
                    Ok(serde_json::Value::Object(map)) => Some(map),
                    Ok(_) => {
                        eprintln!("Error: Arguments must be a JSON object");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error parsing arguments: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                None
            };

            // Call the tool directly
            use codanna::mcp::*;
            use rmcp::handler::server::tool::Parameters;

            let result = match tool.as_str() {
                "find_symbol" => {
                    let name = arguments
                        .as_ref()
                        .and_then(|m| m.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            eprintln!("Error: find_symbol requires 'name' parameter");
                            std::process::exit(1);
                        });
                    server
                        .find_symbol(Parameters(FindSymbolRequest {
                            name: name.to_string(),
                        }))
                        .await
                }
                "get_calls" => {
                    let function_name = arguments
                        .as_ref()
                        .and_then(|m| m.get("function_name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            eprintln!("Error: get_calls requires 'function_name' parameter");
                            std::process::exit(1);
                        });
                    server
                        .get_calls(Parameters(GetCallsRequest {
                            function_name: function_name.to_string(),
                        }))
                        .await
                }
                "find_callers" => {
                    let function_name = arguments
                        .as_ref()
                        .and_then(|m| m.get("function_name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            eprintln!("Error: find_callers requires 'function_name' parameter");
                            std::process::exit(1);
                        });
                    server
                        .find_callers(Parameters(FindCallersRequest {
                            function_name: function_name.to_string(),
                        }))
                        .await
                }
                "analyze_impact" => {
                    let symbol_name = arguments
                        .as_ref()
                        .and_then(|m| m.get("symbol_name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            eprintln!("Error: analyze_impact requires 'symbol_name' parameter");
                            std::process::exit(1);
                        });
                    let max_depth = arguments
                        .as_ref()
                        .and_then(|m| m.get("max_depth"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(3) as usize;
                    server
                        .analyze_impact(Parameters(AnalyzeImpactRequest {
                            symbol_name: symbol_name.to_string(),
                            max_depth,
                        }))
                        .await
                }
                "get_index_info" => server.get_index_info().await,
                "search_symbols" => {
                    let query = arguments
                        .as_ref()
                        .and_then(|m| m.get("query"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            eprintln!("Error: search_symbols requires 'query' parameter");
                            std::process::exit(1);
                        });
                    let limit = arguments
                        .as_ref()
                        .and_then(|m| m.get("limit"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(10) as usize;
                    let kind = arguments
                        .as_ref()
                        .and_then(|m| m.get("kind"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let module = arguments
                        .as_ref()
                        .and_then(|m| m.get("module"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    server
                        .search_symbols(Parameters(SearchSymbolsRequest {
                            query: query.to_string(),
                            limit,
                            kind,
                            module,
                        }))
                        .await
                }
                "semantic_search_docs" => {
                    let query = arguments
                        .as_ref()
                        .and_then(|m| m.get("query"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            eprintln!("Error: semantic_search_docs requires 'query' parameter");
                            std::process::exit(1);
                        });
                    let limit = arguments
                        .as_ref()
                        .and_then(|m| m.get("limit"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(10) as usize;
                    let threshold = arguments
                        .as_ref()
                        .and_then(|m| m.get("threshold"))
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32);
                    server
                        .semantic_search_docs(Parameters(SemanticSearchRequest {
                            query: query.to_string(),
                            limit,
                            threshold,
                        }))
                        .await
                }
                "semantic_search_with_context" => {
                    let query = arguments
                        .as_ref()
                        .and_then(|m| m.get("query"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            eprintln!(
                                "Error: semantic_search_with_context requires 'query' parameter"
                            );
                            std::process::exit(1);
                        });
                    let limit = arguments
                        .as_ref()
                        .and_then(|m| m.get("limit"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(5) as usize;
                    let threshold = arguments
                        .as_ref()
                        .and_then(|m| m.get("threshold"))
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32);
                    server
                        .semantic_search_with_context(Parameters(
                            SemanticSearchWithContextRequest {
                                query: query.to_string(),
                                limit,
                                threshold,
                            },
                        ))
                        .await
                }
                _ => {
                    eprintln!("Unknown tool: {tool}");
                    eprintln!(
                        "Available tools: find_symbol, get_calls, find_callers, analyze_impact, get_index_info, search_symbols, semantic_search_docs, semantic_search_with_context"
                    );
                    std::process::exit(1);
                }
            };

            // Print result
            match result {
                Ok(call_result) => {
                    if let Some(content_vec) = &call_result.content {
                        for content in content_vec {
                            match &**content {
                                rmcp::model::RawContent::Text(text_content) => {
                                    println!("{}", text_content.text);
                                }
                                _ => {
                                    eprintln!("Warning: Non-text content returned");
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error calling tool: {}", e.message);
                    std::process::exit(1);
                }
            }
        }

        Commands::Benchmark { language, file } => {
            run_benchmark_command(&language, file);
        }
    }
}

/// Run parser performance benchmarks
fn run_benchmark_command(language: &str, custom_file: Option<PathBuf>) {
    use codanna::display::theme::Theme;
    use console::style;

    // Print styled header
    if Theme::should_disable_colors() {
        println!("\n=== Codanna Parser Benchmarks ===\n");
    } else {
        println!(
            "\n{}\n",
            style("=== Codanna Parser Benchmarks ===").cyan().bold()
        );
    }

    match language.to_lowercase().as_str() {
        "rust" => benchmark_rust_parser(custom_file),
        "python" => benchmark_python_parser(custom_file),
        "all" => {
            benchmark_rust_parser(None);
            println!();
            benchmark_python_parser(None);
        }
        _ => {
            eprintln!("Unknown language: {language}");
            eprintln!("Available languages: rust, python, all");
            std::process::exit(1);
        }
    }

    // Print target info with styling
    if Theme::should_disable_colors() {
        println!("\nTarget: >10,000 symbols/second ✅");
    } else {
        println!(
            "\n{}: {} ✅",
            style("Target").dim(),
            style(">10,000 symbols/second").dim()
        );
    }
}

fn benchmark_rust_parser(custom_file: Option<PathBuf>) {
    let (code, file_path) = if let Some(path) = custom_file {
        let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Failed to read {}: {e}", path.display());
            std::process::exit(1);
        });
        (content, Some(path))
    } else {
        // Generate benchmark code
        (generate_rust_benchmark_code(), None)
    };

    let mut parser = RustParser::new().expect("Failed to create Rust parser");
    benchmark_parser("Rust", &mut parser, &code, file_path);
}

fn benchmark_python_parser(custom_file: Option<PathBuf>) {
    let (code, file_path) = if let Some(path) = custom_file {
        let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Failed to read {}: {e}", path.display());
            std::process::exit(1);
        });
        (content, Some(path))
    } else if std::path::Path::new("tests/python_comprehensive_features.py").exists() {
        match std::fs::read_to_string("tests/python_comprehensive_features.py") {
            Ok(content) => (content, None),
            Err(e) => {
                eprintln!("Warning: Failed to read test file: {e}");
                eprintln!("Generating benchmark code instead...");
                (generate_python_benchmark_code(), None)
            }
        }
    } else {
        // Generate benchmark code
        (generate_python_benchmark_code(), None)
    };

    let mut parser = PythonParser::new().expect("Failed to create Python parser");
    benchmark_parser("Python", &mut parser, &code, file_path);
}

fn benchmark_parser(
    language: &str,
    parser: &mut dyn LanguageParser,
    code: &str,
    file_path: Option<PathBuf>,
) {
    let file_id = FileId::new(1).expect("Failed to create file ID");
    let mut counter = 1;

    // Warm up
    let _ = parser.parse(code, file_id, &mut counter);

    // Measure parsing performance (average of 3 runs)
    let mut total_duration = std::time::Duration::ZERO;
    let mut symbols_count = 0;

    for _ in 0..3 {
        counter = 1;
        let start = Instant::now();
        let symbols = parser.parse(code, file_id, &mut counter);
        total_duration += start.elapsed();
        symbols_count = symbols.len();
    }

    let avg_duration = total_duration / 3;
    let rate = symbols_count as f64 / avg_duration.as_secs_f64();

    // Display results using rich table
    use codanna::display::tables::create_benchmark_table;

    let table = create_benchmark_table(
        language,
        file_path
            .as_ref()
            .map(|p| p.to_str().unwrap_or("<invalid path>")),
        symbols_count,
        avg_duration,
        rate,
    );

    println!("\n{table}");

    // Verify zero-cost abstractions (silently)
    let calls = parser.find_calls(code);
    if !calls.is_empty() {
        let (caller, _callee, _) = &calls[0];
        let caller_ptr = caller.as_ptr();
        let code_ptr = code.as_ptr();
        let within_bounds =
            caller_ptr >= code_ptr && caller_ptr < unsafe { code_ptr.add(code.len()) };

        if !within_bounds {
            println!("\n⚠️  Warning: String allocation detected!");
        }
    }
}

fn generate_rust_benchmark_code() -> String {
    let mut code = String::from("//! Rust benchmark file\n\n");

    // Generate 500 functions
    for i in 0..500 {
        code.push_str(&format!(
            r#"/// Function {i} documentation
fn function_{i}(param1: i32, param2: &str) -> bool {{
    let result = param1 * 2;
    result > 0
}}

"#
        ));
    }

    // Generate 50 structs with methods
    for i in 0..50 {
        code.push_str(&format!(
            r#"/// Struct {i} documentation
struct Struct{i} {{
    value: i32,
}}

impl Struct{i} {{
    fn new(value: i32) -> Self {{
        Self {{ value }}
    }}
    
    fn method_a(&self) -> i32 {{
        self.value * 2
    }}
}}

"#
        ));
    }

    code
}

fn generate_python_benchmark_code() -> String {
    let mut code = String::from("\"\"\"Python benchmark file\"\"\"\n\n");

    // Generate 500 functions
    for i in 0..500 {
        code.push_str(&format!(
            r#"def function_{i}(param1: int, param2: str = 'default') -> bool:
    """Function {i} documentation."""
    result = param1 * 2
    return result > 0

"#
        ));
    }

    // Generate 50 classes
    for i in 0..50 {
        code.push_str(&format!(
            r#"class Class_{i}:
    """Class {i} documentation."""
    
    def __init__(self, value: int):
        self.value = value
    
    def method_a(self) -> int:
        return self.value * 2

"#
        ));
    }

    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// Verifies CLI structure is valid at compile time.
    ///
    /// Uses clap's debug_assert to catch configuration errors.
    #[test]
    fn verify_cli() {
        // This test ensures the CLI structure is valid
        Cli::command().debug_assert();
    }
}
