//! Python language definition for the registry
//!
//! Provides the Python language implementation that self-registers
//! with the global registry.

use std::sync::Arc;

use crate::{Settings, IndexResult, IndexError};
use super::{
    LanguageId, LanguageDefinition, LanguageParser, LanguageBehavior,
    PythonParser, PythonBehavior,
};

/// Python language definition
pub struct PythonLanguage;

impl PythonLanguage {
    /// Language identifier constant
    pub const ID: LanguageId = LanguageId::new("python");
}

impl LanguageDefinition for PythonLanguage {
    fn id(&self) -> LanguageId {
        Self::ID
    }
    
    fn name(&self) -> &'static str {
        "Python"
    }
    
    fn extensions(&self) -> &'static [&'static str] {
        &["py", "pyi"]
    }
    
    fn create_parser(&self, _settings: &Settings) -> IndexResult<Box<dyn LanguageParser>> {
        let parser = PythonParser::new()
            .map_err(|e| IndexError::General(e.to_string()))?;
        Ok(Box::new(parser))
    }
    
    fn create_behavior(&self) -> Box<dyn LanguageBehavior> {
        Box::new(PythonBehavior::new())
    }
    
    fn is_enabled(&self, settings: &Settings) -> bool {
        settings.languages
            .get(self.id().as_str())
            .map(|config| config.enabled)
            .unwrap_or(true) // Python is enabled by default
    }
}

/// Register Python language with the global registry
pub(super) fn register(registry: &mut super::LanguageRegistry) {
    registry.register(Arc::new(PythonLanguage));
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_python_definition() {
        let python = PythonLanguage;
        
        assert_eq!(python.id(), LanguageId::new("python"));
        assert_eq!(python.name(), "Python");
        assert_eq!(python.extensions(), &["py", "pyi"]);
    }
    
    #[test]
    fn test_python_enabled_by_default() {
        let python = PythonLanguage;
        let settings = Settings::default();
        
        assert!(python.is_enabled(&settings));
    }
}