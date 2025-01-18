use crate::{error::Result, ioutils::path_to_str};
use minijinja::Environment;
use std::path::Path;

/// Trait for template rendering engines.
pub trait TemplateRenderer {
    /// Renders a template string with the given context.
    ///
    /// # Arguments
    /// * `template` - Template string to render
    /// * `context` - Context variables for rendering
    ///
    /// # Returns
    /// * `BakerResult<String>` - Rendered template string
    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String>;
    fn render_path(
        &self,
        template_path: &Path,
        context: &serde_json::Value,
    ) -> Result<String>;
    fn execute_expression(&self, expr: &str, context: &serde_json::Value)
        -> Result<bool>;
}

/// MiniJinja-based template rendering engine.
pub struct MiniJinjaRenderer {
    /// MiniJinja environment instance
    env: Environment<'static>,
}

impl MiniJinjaRenderer {
    /// Creates a new MiniJinjaEngine instance with default environment.
    pub fn new() -> Self {
        let env = Environment::new();
        Self { env }
    }

    fn render_internal(
        &self,
        template: &str,
        context: &serde_json::Value,
    ) -> Result<String> {
        let mut env = self.env.clone();
        env.add_template("temp", template)?;
        let tmpl = env.get_template("temp")?;
        Ok(tmpl.render(context)?)
    }
}

impl Default for MiniJinjaRenderer {
    fn default() -> Self {
        MiniJinjaRenderer::new()
    }
}

impl TemplateRenderer for MiniJinjaRenderer {
    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String> {
        self.render_internal(template, context)
    }
    fn render_path(
        &self,
        template_path: &Path,
        context: &serde_json::Value,
    ) -> Result<String> {
        let path_str = path_to_str(template_path)?;
        self.render_internal(path_str, context)
    }
    fn execute_expression(
        &self,
        expr_str: &str,
        context: &serde_json::Value,
    ) -> Result<bool> {
        let expr = self.env.compile_expression(expr_str)?;
        Ok(expr.eval(context)?.is_true())
    }
}
