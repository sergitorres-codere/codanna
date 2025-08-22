//! Retrieve command implementations using UnifiedOutput schema

use crate::io::{
    EntityType, ExitCode, OutputFormat, OutputManager, OutputStatus,
    schema::{OutputData, OutputMetadata, UnifiedOutput, UnifiedOutputBuilder},
};
use crate::symbol::context::SymbolContext;
use crate::{SimpleIndexer, Symbol};
use std::borrow::Cow;

/// Execute retrieve symbol command
pub fn retrieve_symbol(
    indexer: &SimpleIndexer,
    name: &str,
    language: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(name, language);

    if symbols.is_empty() {
        // Build not found output
        let unified = UnifiedOutput {
            status: OutputStatus::NotFound,
            entity_type: EntityType::Symbol,
            count: 0,
            data: OutputData::<SymbolContext>::Empty,
            metadata: Some(OutputMetadata {
                query: Some(Cow::Borrowed(name)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            }),
            guidance: None,
            exit_code: ExitCode::NotFound,
        };

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    } else {
        // Transform symbols to SymbolContext with file paths and relationships
        use crate::symbol::context::ContextIncludes;

        let symbols_with_path: Vec<SymbolContext> = symbols
            .into_iter()
            .filter_map(|symbol| {
                // Get full context with relationships (same as MCP find_symbol)
                indexer.get_symbol_context(
                    symbol.id,
                    ContextIncludes::IMPLEMENTATIONS
                        | ContextIncludes::DEFINITIONS
                        | ContextIncludes::CALLERS,
                )
            })
            .collect();

        let unified = UnifiedOutputBuilder::items(symbols_with_path, EntityType::Symbol)
            .with_metadata(OutputMetadata {
                query: Some(Cow::Borrowed(name)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            })
            .build();

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    }
}

/// Execute retrieve callers command
pub fn retrieve_callers(
    indexer: &SimpleIndexer,
    function: &str,
    language: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(function, language);

    if symbols.is_empty() {
        let unified = UnifiedOutput {
            status: OutputStatus::NotFound,
            entity_type: EntityType::Function,
            count: 0,
            data: OutputData::<SymbolContext>::Empty,
            metadata: Some(OutputMetadata {
                query: Some(Cow::Borrowed(function)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            }),
            guidance: None,
            exit_code: ExitCode::NotFound,
        };

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    } else {
        let mut all_callers = Vec::new();

        // Check all symbols with this name
        for symbol in &symbols {
            let callers = indexer.get_calling_functions_with_metadata(symbol.id);
            for (caller, _metadata) in callers {
                if !all_callers.iter().any(|c: &Symbol| c.id == caller.id) {
                    all_callers.push(caller);
                }
            }
        }

        // Transform to SymbolContext with relationships
        use crate::symbol::context::ContextIncludes;

        let callers_with_path: Vec<SymbolContext> = all_callers
            .into_iter()
            .filter_map(|symbol| {
                // Get context for each caller symbol (what it calls and defines)
                indexer.get_symbol_context(
                    symbol.id,
                    ContextIncludes::CALLS | ContextIncludes::DEFINITIONS,
                )
            })
            .collect();

        let unified = UnifiedOutputBuilder::items(callers_with_path, EntityType::Function)
            .with_metadata(OutputMetadata {
                query: Some(Cow::Borrowed(function)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            })
            .build();

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    }
}

/// Execute retrieve calls command
pub fn retrieve_calls(
    indexer: &SimpleIndexer,
    function: &str,
    language: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(function, language);

    if symbols.is_empty() {
        let unified = UnifiedOutput {
            status: OutputStatus::NotFound,
            entity_type: EntityType::Function,
            count: 0,
            data: OutputData::<SymbolContext>::Empty,
            metadata: Some(OutputMetadata {
                query: Some(Cow::Borrowed(function)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            }),
            guidance: None,
            exit_code: ExitCode::NotFound,
        };

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    } else {
        let mut all_calls = Vec::new();

        // Collect all unique calls
        for symbol in &symbols {
            let calls = indexer.get_called_functions_with_metadata(symbol.id);
            for (called, _metadata) in calls {
                if !all_calls.iter().any(|c: &Symbol| c.id == called.id) {
                    all_calls.push(called);
                }
            }
        }

        // Transform to SymbolContext with relationships
        use crate::symbol::context::ContextIncludes;

        let calls_with_path: Vec<SymbolContext> = all_calls
            .into_iter()
            .filter_map(|symbol| {
                // Get context for each called function (who calls it, what it defines)
                indexer.get_symbol_context(
                    symbol.id,
                    ContextIncludes::CALLERS | ContextIncludes::DEFINITIONS,
                )
            })
            .collect();

        let unified = UnifiedOutputBuilder::items(calls_with_path, EntityType::Function)
            .with_metadata(OutputMetadata {
                query: Some(Cow::Borrowed(function)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            })
            .build();

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    }
}

/// Execute retrieve implementations command
pub fn retrieve_implementations(
    indexer: &SimpleIndexer,
    trait_name: &str,
    language: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);

    // Find the trait symbol first
    let trait_symbols = indexer.find_symbols_by_name(trait_name, language);
    let implementations = if let Some(trait_symbol) = trait_symbols.first() {
        indexer.get_implementations(trait_symbol.id)
    } else {
        vec![]
    };

    // Transform implementations to SymbolContext with relationships
    use crate::symbol::context::ContextIncludes;

    let impls_with_path: Vec<SymbolContext> = implementations
        .into_iter()
        .filter_map(|symbol| {
            // Get context for each implementation (what it defines, what calls it)
            indexer.get_symbol_context(
                symbol.id,
                ContextIncludes::DEFINITIONS | ContextIncludes::CALLERS,
            )
        })
        .collect();

    let unified = UnifiedOutputBuilder::items(impls_with_path, EntityType::Trait)
        .with_metadata(OutputMetadata {
            query: Some(Cow::Borrowed(trait_name)),
            tool: None,
            timing_ms: None,
            truncated: None,
            extra: Default::default(),
        })
        .build();

    match output.unified(unified) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error writing output: {e}");
            ExitCode::GeneralError
        }
    }
}

/// Execute retrieve search command
pub fn retrieve_search(
    indexer: &SimpleIndexer,
    query: &str,
    limit: usize,
    kind: Option<&str>,
    module: Option<&str>,
    language: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);

    // Parse the kind filter if provided
    let kind_filter = kind.and_then(|k| match k.to_lowercase().as_str() {
        "function" => Some(crate::SymbolKind::Function),
        "struct" => Some(crate::SymbolKind::Struct),
        "trait" => Some(crate::SymbolKind::Trait),
        "interface" => Some(crate::SymbolKind::Interface),
        "class" => Some(crate::SymbolKind::Class),
        "method" => Some(crate::SymbolKind::Method),
        "field" => Some(crate::SymbolKind::Field),
        "variable" => Some(crate::SymbolKind::Variable),
        "constant" => Some(crate::SymbolKind::Constant),
        "module" => Some(crate::SymbolKind::Module),
        "typealias" => Some(crate::SymbolKind::TypeAlias),
        "enum" => Some(crate::SymbolKind::Enum),
        _ => {
            eprintln!("Warning: Unknown symbol kind '{k}', ignoring filter");
            None
        }
    });

    let search_results = indexer
        .search(query, limit, kind_filter, module, language)
        .unwrap_or_default();

    // Transform search results to SymbolContext with relationships
    use crate::symbol::context::ContextIncludes;

    let results_with_path: Vec<SymbolContext> = search_results
        .into_iter()
        .filter_map(|result| {
            // Get full context for each search result
            indexer.get_symbol_context(
                result.symbol_id,
                ContextIncludes::IMPLEMENTATIONS
                    | ContextIncludes::DEFINITIONS
                    | ContextIncludes::CALLERS,
            )
        })
        .collect();

    let unified = UnifiedOutputBuilder::items(results_with_path, EntityType::SearchResult)
        .with_metadata(OutputMetadata {
            query: Some(Cow::Borrowed(query)),
            tool: None,
            timing_ms: None,
            truncated: None,
            extra: Default::default(),
        })
        .build();

    match output.unified(unified) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error writing output: {e}");
            ExitCode::GeneralError
        }
    }
}

/// Execute retrieve impact command
// DEPRECATED: This function has been disabled.
// Use MCP semantic_search_with_context or slash commands instead.
// The impact command had fundamental flaws:
// - Only worked for functions, not structs/traits/enums
// - Returned empty results for valid symbols
// - Conceptually wrong (not all symbols have "impact")
#[allow(dead_code)]
pub fn retrieve_impact(
    indexer: &SimpleIndexer,
    symbol_name: &str,
    max_depth: usize,
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(symbol_name, None);

    if symbols.is_empty() {
        let unified = UnifiedOutput {
            status: OutputStatus::NotFound,
            entity_type: EntityType::Impact,
            count: 0,
            data: OutputData::<SymbolContext>::Empty,
            metadata: Some(OutputMetadata {
                query: Some(Cow::Borrowed(symbol_name)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            }),
            guidance: None,
            exit_code: ExitCode::NotFound,
        };

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    } else {
        // Get impact analysis for the first matching symbol
        let symbol = &symbols[0];
        let impact_symbol_ids = indexer.get_impact_radius(symbol.id, Some(max_depth));

        // Transform impact symbols to SymbolContext with relationships
        use crate::symbol::context::ContextIncludes;

        let impact_with_path: Vec<SymbolContext> = impact_symbol_ids
            .into_iter()
            .filter_map(|symbol_id| {
                // Get full context for each impacted symbol
                indexer.get_symbol_context(
                    symbol_id,
                    ContextIncludes::CALLERS | ContextIncludes::CALLS,
                )
            })
            .collect();

        let unified = UnifiedOutputBuilder::items(impact_with_path, EntityType::Impact)
            .with_metadata(OutputMetadata {
                query: Some(Cow::Borrowed(symbol_name)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            })
            .build();

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    }
}

/// Execute retrieve describe command
pub fn retrieve_describe(
    indexer: &SimpleIndexer,
    symbol_name: &str,
    language: Option<&str>,
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(symbol_name, language);

    if symbols.is_empty() {
        let unified = UnifiedOutput {
            status: OutputStatus::NotFound,
            entity_type: EntityType::Symbol,
            count: 0,
            data: OutputData::<SymbolContext>::Empty,
            metadata: Some(OutputMetadata {
                query: Some(Cow::Borrowed(symbol_name)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            }),
            guidance: None,
            exit_code: ExitCode::NotFound,
        };

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    } else {
        // Get the first matching symbol for basic info, but aggregate relationships from ALL symbols
        let symbol = symbols[0].clone();
        let base_path = indexer
            .get_file_path(symbol.file_id)
            .unwrap_or_else(|| "unknown".to_string());
        let file_path = format!("{}:{}", base_path, symbol.range.start_line + 1);

        // Build context with relationships using the same working methods as retrieve calls/callers
        let mut context = SymbolContext {
            symbol: symbol.clone(),
            file_path,
            relationships: Default::default(),
        };

        // Aggregate calls and callers from ALL symbols with this name (same as retrieve calls/callers)
        let mut all_calls = Vec::new();
        let mut all_callers = Vec::new();

        for sym in &symbols {
            // Collect calls from this symbol
            let calls = indexer.get_called_functions_with_metadata(sym.id);
            for (called, metadata) in calls {
                if !all_calls
                    .iter()
                    .any(|(s, _): &(Symbol, Option<String>)| s.id == called.id)
                {
                    all_calls.push((called, metadata));
                }
            }

            // Collect callers of this symbol
            let callers = indexer.get_calling_functions_with_metadata(sym.id);
            for (caller, metadata) in callers {
                if !all_callers
                    .iter()
                    .any(|(s, _): &(Symbol, Option<String>)| s.id == caller.id)
                {
                    all_callers.push((caller, metadata));
                }
            }
        }

        // Set aggregated relationships
        if !all_calls.is_empty() {
            context.relationships.calls = Some(all_calls);
        }
        if !all_callers.is_empty() {
            context.relationships.called_by = Some(all_callers);
        }

        // Load defines relationships from ALL symbols
        let mut all_defines = Vec::new();
        for sym in &symbols {
            let deps = indexer.get_dependencies(sym.id);
            if let Some(defines) = deps.get(&crate::RelationKind::Defines) {
                for defined in defines {
                    if !all_defines.iter().any(|s: &Symbol| s.id == defined.id) {
                        all_defines.push(defined.clone());
                    }
                }
            }
        }
        if !all_defines.is_empty() {
            context.relationships.defines = Some(all_defines);
        }

        // Load implementations (for traits/interfaces)
        use crate::SymbolKind;
        match symbol.kind {
            SymbolKind::Trait | SymbolKind::Interface => {
                let implementations = indexer.get_implementations(symbol.id);
                if !implementations.is_empty() {
                    context.relationships.implemented_by = Some(implementations);
                }
            }
            _ => {}
        }

        let unified = UnifiedOutput {
            status: OutputStatus::Success,
            entity_type: EntityType::Symbol,
            count: 1,
            data: OutputData::Single {
                item: Box::new(context),
            },
            metadata: Some(OutputMetadata {
                query: Some(Cow::Borrowed(symbol_name)),
                tool: None,
                timing_ms: None,
                truncated: None,
                extra: Default::default(),
            }),
            guidance: None,
            exit_code: ExitCode::Success,
        };

        match output.unified(unified) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error writing output: {e}");
                ExitCode::GeneralError
            }
        }
    }
}
