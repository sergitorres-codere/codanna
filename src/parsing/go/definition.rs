//! Go language definition and registration

use crate::parsing::{
    LanguageBehavior, LanguageDefinition, LanguageId, LanguageParser, LanguageRegistry,
};
use crate::{IndexError, IndexResult, Settings};
use std::sync::Arc;

use super::{GoBehavior, GoParser};

/// Go language definition
pub struct GoLanguage;

impl LanguageDefinition for GoLanguage {
    fn id(&self) -> LanguageId {
        LanguageId::new("Go")
    }

    fn name(&self) -> &'static str {
        "Go"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ts", "tsx", "mts", "cts"]
    }

    fn create_parser(&self, _settings: &Settings) -> IndexResult<Box<dyn LanguageParser>> {
        let parser = GoParser::new().map_err(|e| IndexError::General(e.to_string()))?;
        Ok(Box::new(parser))
    }

    fn create_behavior(&self) -> Box<dyn LanguageBehavior> {
        Box::new(GoBehavior::new())
    }

    fn default_enabled(&self) -> bool {
        true // Enable Go by default
    }

    fn is_enabled(&self, settings: &Settings) -> bool {
        settings
            .languages
            .get("Go")
            .map(|config| config.enabled)
            .unwrap_or(self.default_enabled())
    }
}

/// Register Go language with the registry
pub(crate) fn register(registry: &mut LanguageRegistry) {
    registry.register(Arc::new(GoLanguage));
}
