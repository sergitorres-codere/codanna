//! Custom help formatting for consistent CLI display.
//! This module provides functions to format help text and command descriptions

use crate::display::theme::Theme;
use console::style;

/// Format help text with consistent styling
pub fn format_help_section(title: &str, content: &str, indent: bool) -> String {
    let mut output = String::new();

    // Section header
    if Theme::should_disable_colors() {
        output.push_str(&format!("{title}\n"));
    } else {
        output.push_str(&format!("{}\n", style(title).cyan().bold()));
    }

    // Content with optional indentation
    for line in content.lines() {
        if line.trim().is_empty() {
            output.push('\n');
        } else if indent && !line.starts_with("    ") {
            output.push_str(&format!("    {line}\n"));
        } else {
            output.push_str(&format!("{line}\n"));
        }
    }

    output
}

/// Create styled help text for the CLI
pub fn create_help_text() -> String {
    let mut help = String::new();

    // Quick Start section
    let quick_start = r#"$ codanna init              # Set up in current directory
$ codanna index src         # Index your source code  
$ codanna mcp-test          # Verify Claude can connect"#;

    help.push_str(&format_help_section("QUICK START", quick_start, true));
    help.push('\n');

    // Examples section
    let examples = r#"# First time setup
$ codanna init
$ codanna index src --progress
$ codanna mcp-test

# Index a single file
$ codanna index src/main.rs

# Check what calls your main function  
$ codanna retrieve callers main

# Natural language search (if semantic search enabled)
$ codanna mcp semantic_search_docs --args '{"query": "error handling"}'

# Show detailed loading information
$ codanna --info retrieve symbol main"#;

    help.push_str(&format_help_section("EXAMPLES", examples, true));
    help.push('\n');

    // Benchmarks section
    let benchmarks = r#"# Test parser performance (all languages)
$ codanna benchmark all

# Benchmark specific language
$ codanna benchmark python
$ codanna benchmark rust

# Benchmark with your own file
$ codanna benchmark python --file large_module.py"#;

    help.push_str(&format_help_section("BENCHMARKS", benchmarks, true));
    help.push('\n');

    // Learn More section
    let learn_more = r#"GitHub: https://github.com/bartolli/codanna
Commands: codanna help <COMMAND>"#;

    help.push_str(&format_help_section("LEARN MORE", learn_more, true));

    help
}

/// Format a command description with proper styling
pub fn format_command_description(name: &str, description: &str) -> String {
    if Theme::should_disable_colors() {
        format!("{name:16} {description}")
    } else {
        format!("{:16} {}", style(name).green(), description)
    }
}
