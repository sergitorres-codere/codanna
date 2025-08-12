//! PHP language definition for the registry
//!
//! Provides the PHP language implementation that self-registers
//! with the global registry.

use std::sync::Arc;

use crate::{Settings, IndexResult, IndexError};
use super::{
    LanguageId, LanguageDefinition, LanguageParser, LanguageBehavior,
    PhpParser, PhpBehavior,
};

/// PHP language definition
pub struct PhpLanguage;

impl PhpLanguage {
    /// Language identifier constant
    pub const ID: LanguageId = LanguageId::new("php");
}

impl LanguageDefinition for PhpLanguage {
    fn id(&self) -> LanguageId {
        Self::ID
    }
    
    fn name(&self) -> &'static str {
        "PHP"
    }
    
    fn extensions(&self) -> &'static [&'static str] {
        &["php", "php3", "php4", "php5", "php7", "php8", "phps", "phtml"]
    }
    
    fn create_parser(&self, _settings: &Settings) -> IndexResult<Box<dyn LanguageParser>> {
        let parser = PhpParser::new()
            .map_err(|e| IndexError::General(e.to_string()))?;
        Ok(Box::new(parser))
    }
    
    fn create_behavior(&self) -> Box<dyn LanguageBehavior> {
        Box::new(PhpBehavior::new())
    }
    
    fn is_enabled(&self, settings: &Settings) -> bool {
        settings.languages
            .get(self.id().as_str())
            .map(|config| config.enabled)
            .unwrap_or(true) // PHP is enabled by default
    }
}

/// Register PHP language with the global registry
pub(super) fn register(registry: &mut super::LanguageRegistry) {
    registry.register(Arc::new(PhpLanguage));
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_php_definition() {
        let php = PhpLanguage;
        
        assert_eq!(php.id(), LanguageId::new("php"));
        assert_eq!(php.name(), "PHP");
        assert!(php.extensions().contains(&"php"));
        assert!(php.extensions().contains(&"php5"));
        assert!(php.extensions().contains(&"phtml"));
    }
    
    #[test]
    fn test_php_enabled_by_default() {
        let php = PhpLanguage;
        let settings = Settings::default();
        
        assert!(php.is_enabled(&settings));
    }
}