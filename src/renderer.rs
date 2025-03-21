use crate::{error::Result, ioutils::path_to_str};
use minijinja::Environment;
use serde_json::json;
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
    /// * `Result<String>` - Rendered template string
    fn render(&self, template: &str, context: &serde_json::Value) -> Result<String>;

    /// Renders a path with the given context.
    ///
    /// # Arguments
    /// * `template_path` - Path to render
    /// * `context` - Context variables for rendering
    ///
    /// # Returns
    /// * `Result<String>` - Rendered path as string
    fn render_path(
        &self,
        template_path: &Path,
        context: &serde_json::Value,
    ) -> Result<String>;

    /// Executes a template expression and returns whether it evaluates to true.
    ///
    /// # Arguments
    /// * `expr` - Expression to evaluate
    /// * `context` - Context variables for evaluation
    ///
    /// # Returns
    /// * `Result<bool>` - Whether the expression evaluates to true
    fn execute_expression(&self, expr: &str, context: &serde_json::Value)
        -> Result<bool>;
}

/// MiniJinja-based template rendering engine.
pub struct MiniJinjaRenderer {
    /// MiniJinja environment instance
    env: Environment<'static>,
    /// Default context that will be merged with any provided context
    default_context: serde_json::Value,
}

impl MiniJinjaRenderer {
    /// Creates a new MiniJinjaEngine instance with default environment.
    pub fn new() -> Self {
        let env = Environment::new();
        let default_context = json!({
            "platform": {
                "os": std::env::consts::OS,
                "family": std::env::consts::FAMILY,
                "arch": std::env::consts::ARCH,
            }
        });

        Self { env, default_context }
    }

    /// Internal helper to render templates
    fn render_internal(
        &self,
        template: &str,
        context: &serde_json::Value,
    ) -> Result<String> {
        let mut env = self.env.clone();
        env.add_template("temp", template)?;

        // Merge the default context with the provided context
        let merged_context = if let (Some(default_obj), Some(context_obj)) =
            (self.default_context.as_object(), context.as_object())
        {
            let mut result = default_obj.clone();
            for (key, value) in context_obj {
                result.insert(key.clone(), value.clone());
            }
            json!(result)
        } else {
            // If either isn't an object, just use the provided context
            context.clone()
        };

        let tmpl = env.get_template("temp")?;
        Ok(tmpl.render(merged_context)?)
    }
}

impl Default for MiniJinjaRenderer {
    fn default() -> Self {
        Self::new()
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
        // Only compile the expression if it's not empty
        if expr_str.is_empty() {
            return Ok(true);
        }
        let expr = self.env.compile_expression(expr_str)?;
        Ok(expr.eval(context)?.is_true())
    }
}
