use crate::{error::Result, loader::LoadedTemplate};

/// Trait for loading templates from different sources.
pub trait TemplateLoader {
    /// Loads a template from the given source.
    ///
    /// # Returns
    /// * `Result<LoadedTemplate>` - Loaded template with on-disk path and source metadata
    fn load(&self) -> Result<LoadedTemplate>;
}
