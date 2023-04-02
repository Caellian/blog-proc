use serde::{Deserialize, Serialize};
use std::{convert::Infallible, default::Default, path::Path, str::FromStr, vec};

use chrono::{DateTime, Utc};

use crate::error::{BlogError, FormatError};

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
pub struct PostContent {
    pub inner: String,
}

impl PostContent {
    pub fn new() -> PostContent {
        PostContent {
            inner: String::new(),
        }
    }

    pub fn open(file: impl AsRef<Path>) -> Result<PostContent, BlogError> {
        Ok(PostContent {
            inner: std::fs::read_to_string(file)?,
        })
    }

    pub fn take_info(&mut self) -> Result<PostInfo, BlogError> {
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

impl AsRef<str> for PostContent {
    fn as_ref(&self) -> &str {
        &self.inner
    }
}

impl From<String> for PostContent {
    fn from(inner: String) -> Self {
        PostContent { inner }
    }
}

impl Into<String> for PostContent {
    fn into(self) -> String {
        self.inner
    }
}

impl FromStr for PostContent {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PostContent {
            inner: s.to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    #[serde(flatten)]
    pub info: PostInfo,
    pub content: PostContent,
}

impl Post {}
