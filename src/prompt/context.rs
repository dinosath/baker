use crate::config::Question;

/// Immutable context passed to prompt providers.
///
/// It bundles the configuration question, resolved default value, and
/// the help text that should be rendered for the user.
pub struct PromptContext<'a> {
    pub question: &'a Question,
    pub default: &'a serde_json::Value,
    pub help: &'a str,
}

impl<'a> PromptContext<'a> {
    pub fn new(
        question: &'a Question,
        default: &'a serde_json::Value,
        help: &'a str,
    ) -> Self {
        Self { question, default, help }
    }
}
