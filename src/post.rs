use std::{convert::Infallible, default::Default, path::Path, str::FromStr, vec};

use chrono::{DateTime, Utc};
use render::Render;
use serde::{Deserialize, Serialize};

use crate::{
    component::Parser,
    error::{BlogError, FormatError},
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Edit {
    pub summary: String,
    pub time: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
    pub web: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuthorEntry {
    Name(String),
    Author(Author),
    AuthorList(Vec<Author>),
}

impl Default for AuthorEntry {
    fn default() -> Self {
        AuthorEntry::Name(String::default())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PostInfo {
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub slug: Option<String>,
    pub author: Option<AuthorEntry>,
    pub edits: Option<Vec<Edit>>,
}

impl PostInfo {
    pub fn new() -> PostInfo {
        PostInfo {
            title: Some("Blog article".to_string()),
            description: None,
            tags: vec![],
            slug: None,
            author: None,
            edits: None,
        }
    }
}

pub trait MergeData<With> {
    fn merge_replace(&mut self, value: With);
}

impl MergeData<PostInfo> for PostInfo {
    fn merge_replace(&mut self, value: PostInfo) {
        if let Some(it) = value.description {
            self.description = Some(it);
        }
        if value.tags.len() > 0 {
            self.tags = value.tags;
        }
        if let Some(it) = value.slug {
            self.slug = Some(it);
        }
        if let Some(it) = value.author {
            self.author = Some(it);
        }
        if let Some(it) = value.edits {
            self.edits = Some(it);
        }
    }
}

impl FromStr for PostInfo {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct RawPostContent {
    pub inner: String,
}

impl RawPostContent {
    pub fn new() -> RawPostContent {
        RawPostContent {
            inner: String::new(),
        }
    }

    pub fn open(file: impl AsRef<Path>) -> Result<RawPostContent, BlogError> {
        Ok(RawPostContent {
            inner: std::fs::read_to_string(file)?,
        })
    }

    pub(crate) fn take_info(&mut self) -> Result<PostInfo, BlogError> {
        if self.inner.len() <= 8 {
            return Ok(PostInfo::default());
        }

        let frontmatter_start = self
            .inner
            .chars()
            .enumerate()
            .skip_while(|it| it.1.is_whitespace())
            .next()
            .map(|it| it.0 + 4)
            .unwrap_or(4);

        if !self.inner[frontmatter_start - 4..].starts_with("---\n") {
            return Ok(PostInfo::default());
        }

        let frontmatter_end = frontmatter_start
            + self.inner[(frontmatter_start)..]
                .find("---\n")
                .ok_or(BlogError::Format(FormatError::UnclosedFrontmatter))?;

        let frontmatter = &self.inner[frontmatter_start..frontmatter_end];
        let result = PostInfo::from_str(frontmatter).unwrap_or_default();

        let content = &self.inner[(frontmatter_end + 4)..];
        self.inner = content.to_string();

        Ok(result)
    }
}

impl AsRef<str> for RawPostContent {
    fn as_ref(&self) -> &str {
        &self.inner
    }
}

impl From<String> for RawPostContent {
    fn from(inner: String) -> Self {
        RawPostContent { inner }
    }
}

impl Into<String> for RawPostContent {
    fn into(self) -> String {
        self.inner
    }
}

impl FromStr for RawPostContent {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(RawPostContent {
            inner: s.to_string(),
        })
    }
}

#[derive(Debug)]
pub struct Post {
    pub info: PostInfo,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostTemplateContext {
    #[serde(flatten)]
    pub info: PostInfo,
    pub content: String,
}

impl Post {
    pub fn new(mut raw: RawPostContent) -> Result<Self, BlogError> {
        Ok(Post {
            info: raw.take_info()?,
            source: raw.inner,
        })
    }

    pub fn components(&self) -> Parser {
        Parser::new(&self.source)
    }

    pub fn template_ctx(self) -> PostTemplateContext {
        let mut content = String::with_capacity(1024);

        for c in self.components() {
            c.render_into(&mut content)
                .expect("post component render should be infallible");
        }

        content.shrink_to_fit();

        PostTemplateContext {
            info: self.info,
            content,
        }
    }
}
