//! Retrieve command implementations using UnifiedOutput schema

use crate::io::{
    EntityType, ExitCode, OutputFormat, OutputManager, OutputStatus,
    schema::{OutputData, OutputMetadata, UnifiedOutput, UnifiedOutputBuilder},
};
use crate::symbol::context::SymbolContext;
use crate::{SimpleIndexer, Symbol};
use std::borrow::Cow;

/// Execute retrieve symbol command
pub fn retrieve_symbol(indexer: &SimpleIndexer, name: &str, format: OutputFormat) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(name);

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
        // Transform symbols to SymbolContext with file paths
        let symbols_with_path: Vec<SymbolContext> = symbols
            .into_iter()
            .map(|symbol| {
                let base_path = indexer
                    .get_file_path(symbol.file_id)
                    .unwrap_or_else(|| "unknown".to_string());
                let file_path = format!("{}:{}", base_path, symbol.range.start_line + 1);
                SymbolContext {
                    symbol,
                    file_path,
                    relationships: Default::default(),
                }
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
pub fn retrieve_callers(indexer: &SimpleIndexer, function: &str, format: OutputFormat) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(function);

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

        // Transform to SymbolContext
        let callers_with_path: Vec<SymbolContext> = all_callers
            .into_iter()
            .map(|symbol| {
                let base_path = indexer
                    .get_file_path(symbol.file_id)
                    .unwrap_or_else(|| "unknown".to_string());
                let file_path = format!("{}:{}", base_path, symbol.range.start_line + 1);
                SymbolContext {
                    symbol,
                    file_path,
                    relationships: Default::default(),
                }
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
pub fn retrieve_calls(indexer: &SimpleIndexer, function: &str, format: OutputFormat) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(function);

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

        // Transform to SymbolContext
        let calls_with_path: Vec<SymbolContext> = all_calls
            .into_iter()
            .map(|symbol| {
                let base_path = indexer
                    .get_file_path(symbol.file_id)
                    .unwrap_or_else(|| "unknown".to_string());
                let file_path = format!("{}:{}", base_path, symbol.range.start_line + 1);
                SymbolContext {
                    symbol,
                    file_path,
                    relationships: Default::default(),
                }
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
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);

    // Find the trait symbol first
    let trait_symbols = indexer.find_symbols_by_name(trait_name);
    let implementations = if let Some(trait_symbol) = trait_symbols.first() {
        indexer.get_implementations(trait_symbol.id)
    } else {
        vec![]
    };

    let impls_with_path: Vec<SymbolContext> = implementations
        .into_iter()
        .map(|symbol| {
            let base_path = indexer
                .get_file_path(symbol.file_id)
                .unwrap_or_else(|| "unknown".to_string());
            let file_path = format!("{}:{}", base_path, symbol.range.start_line + 1);
            SymbolContext {
                symbol,
                file_path,
                relationships: Default::default(),
            }
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
        .search(query, limit, kind_filter, module)
        .unwrap_or_default();

    let results_with_path: Vec<SymbolContext> = search_results
        .into_iter()
        .filter_map(|result| {
            // Convert SearchResult to Symbol
            if let Some(symbol) = indexer.get_symbol(result.symbol_id) {
                let file_path = format!("{}:{}", result.file_path, result.line + 1);
                Some(SymbolContext {
                    symbol,
                    file_path,
                    relationships: Default::default(),
                })
            } else {
                None
            }
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
pub fn retrieve_impact(
    indexer: &SimpleIndexer,
    symbol_name: &str,
    max_depth: usize,
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(symbol_name);

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
        // Get impact analysis for the first matching symbol
        let symbol = &symbols[0];
        let impact_symbol_ids = indexer.get_impact_radius(symbol.id, Some(max_depth));

        let impact_with_path: Vec<SymbolContext> = impact_symbol_ids
            .into_iter()
            .filter_map(|symbol_id| {
                // Get the actual symbol from the ID
                if let Some(symbol) = indexer.get_symbol(symbol_id) {
                    let base_path = indexer
                        .get_file_path(symbol.file_id)
                        .unwrap_or_else(|| "unknown".to_string());
                    let file_path = format!("{}:{}", base_path, symbol.range.start_line + 1);
                    Some(SymbolContext {
                        symbol,
                        file_path,
                        relationships: Default::default(),
                    })
                } else {
                    None
                }
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
    format: OutputFormat,
) -> ExitCode {
    let mut output = OutputManager::new(format);
    let symbols = indexer.find_symbols_by_name(symbol_name);

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
        // Get the first matching symbol and describe it
        let symbol = symbols[0].clone();
        let base_path = indexer
            .get_file_path(symbol.file_id)
            .unwrap_or_else(|| "unknown".to_string());
        let file_path = format!("{}:{}", base_path, symbol.range.start_line + 1);

        // Get relationships
        let callers = indexer.get_calling_functions_with_metadata(symbol.id);
        let calls = indexer.get_called_functions_with_metadata(symbol.id);
        let implementations = if symbol.kind == crate::SymbolKind::Trait {
            indexer.get_implementations(symbol.id)
        } else {
            vec![]
        };

        // Build contextual data with all relationships
        let mut context = SymbolContext {
            symbol: symbol.clone(),
            file_path,
            relationships: Default::default(),
        };

        // Add relationships
        if !callers.is_empty() {
            let mut called_by = Vec::new();
            for (caller, metadata) in callers {
                called_by.push((caller, metadata));
            }
            context.relationships.called_by = Some(called_by);
        }

        if !calls.is_empty() {
            let mut calls_list = Vec::new();
            for (called, metadata) in calls {
                calls_list.push((called, metadata));
            }
            context.relationships.calls = Some(calls_list);
        }

        if !implementations.is_empty() {
            context.relationships.implemented_by = Some(implementations);
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
