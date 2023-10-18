use std::path::Path;

use serde::Serialize;

use crate::error::FormatError;

use super::Generate;

#[derive(Debug, Serialize)]
pub struct RedirectTemplate {
    pub target: String,
    pub delay: String,
    pub head: Option<String>,
}

impl Generate for RedirectTemplate {
    fn generate(&self, path: impl AsRef<Path>) -> Result<String, FormatError> {
        let engine = super::engine().read().expect("engine poisoned");
        let contents = engine.render("redirect", &self)?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &contents)?;
        Ok(contents)
    }
}

impl Default for RedirectTemplate {
    fn default() -> Self {
        RedirectTemplate {
            target: "#".to_string(),
            delay: "3".to_string(),
            head: None,
        }
    }
}
