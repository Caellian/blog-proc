use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Redirect {
    pub target: String,
    pub delay: String,
    pub head: Option<String>,
}

impl Redirect {
    pub fn generate_nbs(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let mut reg = Handlebars::new();
        let template = std::fs::read_to_string("std/redirect.hbs")?;

        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = reg.render_template(&template, &self)?;
        std::fs::write(path, &contents);
        Ok(contents)
    }
}

impl Default for Redirect {
    fn default() -> Self {
        Redirect {
            target: "#".to_string(),
            delay: "3".to_string(),
            head: None,
        }
    }
}
