//! Language parser factory with configuration-based instantiation.
//!
//! Creates LanguageParser instances based on Language enum and Settings.
//! Validates language enablement and provides discovery of supported languages.

use super::{
    Language, LanguageBehavior, LanguageParser, 
    PhpParser, PythonParser, RustParser
};
use crate::parsing::{
    php_behavior::PhpBehavior,
    python_behavior::PythonBehavior,
    rust_behavior::RustBehavior,
};
use crate::{IndexError, IndexResult, Settings};
use std::sync::Arc;

/// A parser paired with its language-specific behavior
pub struct ParserWithBehavior {
    pub parser: Box<dyn LanguageParser>,
    pub behavior: Box<dyn LanguageBehavior>,
}

/// Parser factory that creates LanguageParser instances based on configuration.
///
/// Validates language support and configuration before instantiation.
/// Currently supports: Rust (full), Python/JS/TS (placeholder).
#[derive(Debug)]
pub struct ParserFactory {
    settings: Arc<Settings>,
}

impl ParserFactory {
    /// Creates factory instance with shared configuration.
    ///
    /// Args: settings - Arc-wrapped Settings for thread-safe access across parsing tasks.
    pub fn new(settings: Arc<Settings>) -> Self {
        Self { settings }
    }

    /// Creates parser instance for the specified language.
    ///
    /// Validates language is enabled in configuration before creation.
    /// Returns ConfigError if language disabled, General error if unimplemented.
    #[must_use = "Parser creation may fail and should be handled"]
    pub fn create_parser(&self, language: Language) -> IndexResult<Box<dyn LanguageParser>> {
        // Validate language enablement before expensive parser creation
        let lang_key = language.config_key();
        if let Some(config) = self.settings.languages.get(lang_key) {
            if !config.enabled {
                return Err(IndexError::ConfigError {
                    reason: format!(
                        "Language {} is disabled in configuration. Enable it in your settings to use.",
                        language.name()
                    ),
                });
            }
        }

        match language {
            Language::Rust => {
                let parser =
                    RustParser::with_debug(self.settings.debug).map_err(IndexError::General)?;
                Ok(Box::new(parser))
            }
            Language::Python => {
                let parser = PythonParser::new().map_err(|e| IndexError::General(e.to_string()))?;
                Ok(Box::new(parser))
            }
            Language::JavaScript => {
                // TODO: Implement JavaScriptParser
                Err(IndexError::General(format!(
                    "{} parser not yet implemented. Currently only Rust is supported.",
                    language.name()
                )))
            }
            Language::TypeScript => {
                // TODO: Implement TypeScriptParser
                Err(IndexError::General(format!(
                    "{} parser not yet implemented. Currently only Rust is supported.",
                    language.name()
                )))
            }
            Language::Php => {
                let parser = PhpParser::new().map_err(|e| IndexError::General(e.to_string()))?;
                Ok(Box::new(parser))
            }
        }
    }

    /// Checks if language is enabled in configuration.
    ///
    /// Returns false if language not found in settings (fail-safe default).
    pub fn is_language_enabled(&self, language: Language) -> bool {
        let lang_key = language.config_key();
        self.settings
            .languages
            .get(lang_key)
            .map(|config| config.enabled)
            .unwrap_or(false)
    }

    /// Creates a parser with its corresponding language behavior.
    ///
    /// This method pairs each parser with its language-specific behavior,
    /// enabling the removal of hardcoded language logic from indexing.
    pub fn create_parser_with_behavior(&self, language: Language) -> IndexResult<ParserWithBehavior> {
        // Validate language enablement
        let lang_key = language.config_key();
        if let Some(config) = self.settings.languages.get(lang_key) {
            if !config.enabled {
                return Err(IndexError::ConfigError {
                    reason: format!(
                        "Language {} is disabled in configuration. Enable it in your settings to use.",
                        language.name()
                    ),
                });
            }
        }

        // Create parser and behavior pair
        let result = match language {
            Language::Rust => {
                let parser = RustParser::with_debug(self.settings.debug)
                    .map_err(IndexError::General)?;
                ParserWithBehavior {
                    parser: Box::new(parser),
                    behavior: Box::new(RustBehavior::new()),
                }
            }
            Language::Python => {
                let parser = PythonParser::new()
                    .map_err(|e| IndexError::General(e.to_string()))?;
                ParserWithBehavior {
                    parser: Box::new(parser),
                    behavior: Box::new(PythonBehavior::new()),
                }
            }
            Language::Php => {
                let parser = PhpParser::new()
                    .map_err(|e| IndexError::General(e.to_string()))?;
                ParserWithBehavior {
                    parser: Box::new(parser),
                    behavior: Box::new(PhpBehavior::new()),
                }
            }
            Language::JavaScript | Language::TypeScript => {
                return Err(IndexError::General(format!(
                    "{} parser not yet implemented.",
                    language.name()
                )));
            }
        };

        Ok(result)
    }

    /// Returns list of all enabled languages from configuration.
    ///
    /// Filters all supported languages against settings.languages map.
    pub fn enabled_languages(&self) -> Vec<Language> {
        vec![
            Language::Rust,
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
            Language::Php,
        ]
        .into_iter()
        .filter(|&lang| self.is_language_enabled(lang))
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_rust_parser() {
        let settings = Arc::new(Settings::default());
        let factory = ParserFactory::new(settings);

        let parser = factory.create_parser(Language::Rust);
        assert!(parser.is_ok());

        let parser = parser.unwrap();
        assert_eq!(parser.language(), Language::Rust);
    }

    #[test]
    fn test_create_parser_with_behavior() {
        use crate::config::LanguageConfig;
        use std::collections::HashMap;

        // Create settings with all languages enabled
        let mut settings = Settings::default();
        let mut languages = HashMap::new();
        
        // Enable Rust (already enabled by default, but be explicit)
        languages.insert(
            "rust".to_string(),
            LanguageConfig {
                enabled: true,
                extensions: vec!["rs".to_string()],
                parser_options: HashMap::new(),
            },
        );
        
        // Enable Python
        languages.insert(
            "python".to_string(),
            LanguageConfig {
                enabled: true,
                extensions: vec!["py".to_string()],
                parser_options: HashMap::new(),
            },
        );
        
        // Enable PHP
        languages.insert(
            "php".to_string(),
            LanguageConfig {
                enabled: true,
                extensions: vec!["php".to_string()],
                parser_options: HashMap::new(),
            },
        );
        
        settings.languages = languages;
        let factory = ParserFactory::new(Arc::new(settings));

        // Test Rust
        let result = factory.create_parser_with_behavior(Language::Rust);
        assert!(result.is_ok());
        let rust_pair = result.unwrap();
        assert_eq!(rust_pair.parser.language(), Language::Rust);
        assert_eq!(rust_pair.behavior.module_separator(), "::");
        
        // Test Python
        let result = factory.create_parser_with_behavior(Language::Python);
        assert!(result.is_ok());
        let python_pair = result.unwrap();
        assert_eq!(python_pair.parser.language(), Language::Python);
        assert_eq!(python_pair.behavior.module_separator(), ".");
        
        // Test PHP
        let result = factory.create_parser_with_behavior(Language::Php);
        assert!(result.is_ok());
        let php_pair = result.unwrap();
        assert_eq!(php_pair.parser.language(), Language::Php);
        assert_eq!(php_pair.behavior.module_separator(), "\\");
    }

    #[test]
    fn test_create_parser_with_behavior_disabled_language() {
        let mut settings = Settings::default();
        // Disable Rust
        if let Some(rust_config) = settings.languages.get_mut("rust") {
            rust_config.enabled = false;
        }

        let factory = ParserFactory::new(Arc::new(settings));
        let result = factory.create_parser_with_behavior(Language::Rust);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(
                matches!(err, IndexError::ConfigError { reason } if reason.contains("disabled"))
            );
        }
    }

    #[test]
    fn test_disabled_language() {
        let mut settings = Settings::default();
        // Disable Rust
        if let Some(rust_config) = settings.languages.get_mut("rust") {
            rust_config.enabled = false;
        }

        let factory = ParserFactory::new(Arc::new(settings));
        let result = factory.create_parser(Language::Rust);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(
                matches!(err, IndexError::ConfigError { reason } if reason.contains("disabled"))
            );
        }
    }

    #[test]
    fn test_enabled_languages() {
        let settings = Arc::new(Settings::default());
        let factory = ParserFactory::new(settings);

        let enabled = factory.enabled_languages();
        // By default, only Rust is enabled
        assert_eq!(enabled, vec![Language::Rust]);
    }

    #[test]
    fn test_create_python_parser() {
        use crate::config::LanguageConfig;
        use std::collections::HashMap;

        let mut settings = Settings::default();
        let mut languages = HashMap::new();
        languages.insert(
            "python".to_string(),
            LanguageConfig {
                enabled: true,
                extensions: vec!["py".to_string()],
                parser_options: HashMap::new(),
            },
        );
        settings.languages = languages;

        let factory = ParserFactory::new(Arc::new(settings));
        let parser = factory.create_parser(Language::Python);

        assert!(parser.is_ok());
        let parser = parser.unwrap();
        assert_eq!(parser.language(), Language::Python);
    }
}
