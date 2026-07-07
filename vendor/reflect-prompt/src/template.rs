//! `template` — minijinja wrapper for `{{ var }}` substitution.
//!
//! Reflect's prompt manager uses Jinja2 substitution; reflect uses
//! minijinja (which supports the Jinja2 syntax natively). v0 only does
//! variable substitution — no `{% if %}` / `{% for %}` blocks (matches
//! reflect `manager.get_prompt`).

use minijinja::{Environment, Value};
use thiserror::Error;

/// Errors that can occur while rendering a prompt template.
#[derive(Debug, Error)]
pub enum PromptError {
    /// minijinja reported a syntax / evaluation error.
    #[error("template render error: {0}")]
    Render(String),
}

/// Render `template` against the given JSON object. Only `{{ var }}`
/// substitution is supported; missing variables become the empty string
/// in non-strict mode (this matches reflect's tolerance for empty values
/// in `{{ task_content }}`-style placeholders).
pub fn render(template: &str, vars: &serde_json::Value) -> Result<String, PromptError> {
    let value: Value = serde_json::from_str(&serde_json::to_string(vars).unwrap_or_default())
        .unwrap_or(Value::UNDEFINED);
    let mut env = Environment::new();
    env.set_trim_blocks(true);
    env.set_lstrip_blocks(true);
    // Undefined defaults: missing variables become "" so reflect-style
    // templates like "Workspace: {{ workspace }}" still render cleanly
    // when no workspace was provided.
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Lenient);
    let tmpl = env
        .template_from_str(template)
        .map_err(|e| PromptError::Render(e.to_string()))?;
    tmpl.render(value)
        .map_err(|e| PromptError::Render(e.to_string()))
}

/// Same as [`render`] but with explicit context — convenient when the
/// caller has a `serde_json::Map` rather than a `Value`.
pub fn render_with<S: serde::Serialize>(template: &str, vars: S) -> Result<String, PromptError> {
    let value = serde_json::to_value(vars).unwrap_or(serde_json::Value::Null);
    render(template, &value)
}

/// Convenience for the most common case: a single `{{ task_content }}`
/// placeholder. Equivalent to `render(template, &serde_json::json!{"task_content": ...})`.
pub fn render_task_content(template: &str, task_content: &str) -> Result<String, PromptError> {
    render(
        template,
        &serde_json::json!({ "task_content": task_content }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_substitutes_simple_var() {
        let out = render("Hello {{ name }}", &serde_json::json!({"name": "world"})).unwrap();
        assert_eq!(out, "Hello world");
    }

    #[test]
    fn render_handles_missing_var_lenient() {
        let out = render("Hello {{ name }}", &serde_json::json!({})).unwrap();
        assert_eq!(out, "Hello ");
    }

    #[test]
    fn render_with_object() {
        let out = render_with("a={{a}}, b={{b}}", serde_json::json!({"a": 1, "b": 2})).unwrap();
        assert_eq!(out, "a=1, b=2");
    }

    #[test]
    fn render_task_content_works() {
        let out = render_task_content("Task: {{ task_content }}", "do the thing").unwrap();
        assert_eq!(out, "Task: do the thing");
    }

    #[test]
    fn render_plain_text_passes_through() {
        let out = render("no vars here", &serde_json::json!({})).unwrap();
        assert_eq!(out, "no vars here");
    }

    #[test]
    fn render_trims_around_tags() {
        // trim_blocks + lstrip_blocks: `{% if %}` is a tag block, so
        // lstrip strips whitespace before the block and trim strips the
        // newline after the block.
        let out = render(
            "header\n  {% if true %}X{% endif %}\nfooter",
            &serde_json::json!({}),
        )
        .unwrap();
        // After trimming: "header\n" + "X" + "footer" → "header\nXfooter"
        assert_eq!(out, "header\nXfooter");
    }

    #[test]
    fn render_error_on_invalid_template() {
        // Unmatched brace = syntax error.
        let err = render("{{ unclosed", &serde_json::json!({})).unwrap_err();
        match err {
            PromptError::Render(_) => {}
        }
    }
}
