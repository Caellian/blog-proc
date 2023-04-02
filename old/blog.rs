use core::str;
use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt::{Debug, Display},
    path::{Path, PathBuf},
    str::FromStr,
};

use chrono::{DateTime, NaiveDate, ParseError, Utc};
use clap::Args;
use git2::{build::RemoteCreate, Remote, Repository};
use nym::glob::Glob;
use serde::{Deserialize, Serialize};
use serde_yaml::Mapping;

use crate::{
    arguments::GitSource,
    error::{BlogError, FormatError},
    git::ExtRepository,
    parser::{process_markdown, HTML},
    util::MinMax,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PostInfo {
    pub title: String,
    pub description: String,

    pub slug: String,

    pub tags: Vec<String>,

    pub published: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl PostInfo {
    pub fn new(path: impl Into<PathBuf>) -> Result<PostInfo, FormatError> {
        let path = path.into();
        let content = std::fs::read_to_string(path)?;

        let split: Vec<&str> = content.split("---").map(|s| s.trim()).collect();

        if split.len() == 1 {
            return Err(FormatError::MissingHeader);
        }

        let info = match split.len() {
            1 => None,
            2 => {
                if let Ok(info) = PostInfo::from_str(split[0]) {
                    Some(info)
                } else {
                    None
                }
            }
            _ => split
                .iter()
                .enumerate()
                .skip_while(|(i, it)| *i == 0 && it.len() == 0)
                .find_map(|(i, source)| PostInfo::from_str(source).ok()),
            0 => unreachable!(),
        }
        .ok_or(FormatError::MissingHeader);

        let info = info.unwrap_or_default();
        Ok(info)
    }
}

pub async fn read_post_file(content: &str) -> Result<(PostInfo, HTML), FormatError> {
    let split: Vec<&str> = content.split("---").map(|s| s.trim()).collect();

    let (info, md) = match split.len() {
        1 => (None, content),
        2 => {
            if let Ok(info) = PostInfo::from_str(split[0]) {
                (Some(info), split[1])
            } else {
                (None, content)
            }
        }
        _ => split
            .iter()
            .enumerate()
            .skip_while(|(_, it)| it.len() == 0)
            .find_map(|(i, source)| {
                PostInfo::from_str(source).ok().map(|info| {
                    (
                        Some(info),
                        content
                            .split_at(
                                split
                                    .iter()
                                    .enumerate()
                                    .take_while(|(j, _)| *j <= i)
                                    .map(|(_, el)| el.len())
                                    .sum(),
                            )
                            .1,
                    )
                })
            })
            .unwrap_or((None, content)),
        0 => unreachable!(),
    };

    Ok((info.unwrap_or_default(), process_markdown(md).await?))
}

impl FromStr for PostInfo {
    type Err = FormatError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let mut result = PostInfo::default();
        let src = source.trim();
        let val: Mapping = serde_yaml::from_str(src)?;

        for (k, v) in val.iter() {
            if let Some(key) = k.as_str() {
                match key.to_ascii_lowercase().as_str() {
                    "title" => {
                        if let Some(val) = v.as_str() {
                            result.title = val.to_string();
                        }
                    }
                    "description" | "desc" => {
                        if let Some(val) = v.as_str() {
                            result.description = val.to_string();
                        }
                    }
                    "tags" => {
                        if let Some(val) = v.as_sequence() {
                            result.tags = val
                                .iter()
                                .filter_map(|it| it.as_str().map(|s| s.to_string()))
                                .collect();
                        }
                    }
                    "slug" => {
                        if let Some(val) = v.as_str() {
                            result.slug = val.to_string();
                        }
                    }
                    "published" => {
                        if let Some(val) = v.as_str() {
                            result.published = read_date(val.to_string().as_str())?;
                        }
                    }
                    "last_updated" | "lastUpdated" | "updated" => {
                        if let Some(val) = v.as_str() {
                            result.last_updated = read_date(val.to_string().as_str())?;
                        }
                    }
                    _ => {
                        log::warn!("Ignoring unused header attribute: '{}'", key);
                    }
                }
            }
        }

        Ok(result)
    }
}

fn read_date(source: &str) -> Result<DateTime<Utc>, FormatError> {
    source
        .parse::<DateTime<Utc>>()
        .map_err(|err| FormatError::DateTimeParse {
            invalid: source.to_string(),
            source: err,
        })
}

pub struct Post {
    pub info: PostInfo,
    pub content: HTML,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Field {
    pub name: &'static str,
}

impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Field {
    pub const TITLE_FIELD: Field = Field { name: "title" };
    pub const DESCRIPTION_FIELD: Field = Field {
        name: "description",
    };
    pub const SLUG_FIELD: Field = Field { name: "slug" };
    pub const TAGS_FIELD: Field = Field { name: "tags" };
    pub const PUBLISHED_FIELD: Field = Field { name: "published" };
    pub const LAST_UPDATE_FIELD: Field = Field {
        name: "last_updated",
    };
    pub const CONTENT_FIELD: Field = Field { name: "content" };
}

#[derive(Debug, Clone, Args)]
pub struct PostQuery {
    /// Posts newer than start date will be included
    #[arg(short = 's', long = "start")]
    pub start_date: Option<NaiveDate>,
    /// Posts older than end date will be included
    #[arg(short = 'e', long = "end")]
    pub end_date: Option<NaiveDate>,
    /// Posts containing all of comma separated tags will be included
    #[arg(short = 't', long = "tags")]
    pub tags: Option<String>,
    /// Literal text contained within a blog post
    #[arg(short = 'q', long = "text-query")]
    pub content: Option<String>,
}

impl PostQuery {
    pub fn is_empty(&self) -> bool {
        self.start_date.is_none()
            && self.end_date.is_none()
            && self.tags.is_none()
            && self.content.is_none()
    }

    pub fn iter_tags(&self) -> Option<impl Iterator<Item = &str> + '_> {
        self.tags.as_ref().map(|t| t.split(','))
    }
}

#[derive(Debug, Clone, Hash)]
pub struct IndexData {
    created: Option<DateTime<Utc>>,
    modified: Option<DateTime<Utc>>,
}

impl Default for IndexData {
    fn default() -> Self {
        IndexData {
            created: None,
            modified: None,
        }
    }
}

impl IndexData {
    pub fn new(path: impl AsRef<Path>, repository: Option<&Repository>) -> IndexData {
        let path = path.as_ref();

        let mut created = if let Ok(metadata) = path.metadata() {
            metadata.created().ok().map(|time| DateTime::from(time))
        } else {
            None
        };

        let mut modified = if let Ok(metadata) = path.metadata() {
            metadata.modified().ok().map(|time| DateTime::from(time))
        } else {
            None
        };

        if let Some(min_max) = repository.map(|it| {
            it.file_history(it.workdir().unwrap_or_else(|| Path::new(".")))
                .into()
        }) {
            match min_max {
                MinMax::One(create) => {
                    created = created.map(|it| create.min(it)).or(Some(create));
                }
                MinMax::Complete {
                    min: create,
                    max: modify,
                } => {
                    created = created.map(|it| create.min(it)).or(Some(create));
                    modified = created.map(|it| modify.min(it)).or(Some(modify));
                }
                MinMax::Empty => {}
            }
        }

        IndexData { created, modified }
    }
}

#[derive(Debug)]
pub struct FileIndex {
    files: HashMap<PathBuf, IndexData, fasthash::city::Hash128>,
}

impl FileIndex {
    pub fn new() -> FileIndex {
        FileIndex {
            files: HashMap::with_capacity_and_hasher(1024, fasthash::city::Hash128),
        }
    }

    pub fn note(&mut self, file: impl AsRef<Path>, data: Option<IndexData>) {
        let file = file.as_ref();
        self.files
            .insert(file.to_path_buf(), data.unwrap_or_default());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &IndexData)> + '_ {
        self.files.iter()
    }
}

pub struct Blog {
    source_dir: PathBuf,

    repo: Option<Repository>,
    file_index: Option<FileIndex>,
}

impl Debug for Blog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Blog")
            .field("source_dir", &self.source_dir)
            .field("file_index", &self.file_index)
            .finish()
    }
}

impl Default for Blog {
    fn default() -> Self {
        Blog {
            source_dir: PathBuf::new(),
            repo: None,
            file_index: None,
        }
    }
}

impl Blog {
    pub fn open(root: impl AsRef<Path>) -> Result<Blog, BlogError> {
        let root = root.as_ref();
        let source_dir = root.to_path_buf();

        if root.is_dir() {
            if let Ok(repo) = git2::Repository::open(root) {
                Ok(Blog {
                    source_dir,
                    repo: Some(repo),
                    ..Default::default()
                })
            } else {
                Ok(Blog {
                    source_dir,
                    ..Default::default()
                })
            }
        } else {
            Err(BlogError::InvalidRoot(source_dir))
        }
    }

    pub fn index_files(&mut self) -> &FileIndex {
        if self.file_index.is_some() {
            return self.file_index.as_ref().unwrap();
        }

        self.file_index.get_or_insert_with(|| {
            let mut file_index = FileIndex::new();

            let glob = Glob::new("**/*.md").unwrap();
            for entry in glob.read(&self.source_dir, 25) {
                let path = entry.unwrap().path().to_path_buf();
                file_index.note(path, Some(IndexData::new(path, self.repo.as_ref())));
            }

            file_index
        })
    }

    pub fn pull(&self) {
        match self.repo {
            Some(repo) => {
                // repo.fetchhead_foreach(callback);
                // let ds = repo.checkout_head(opts);
                // let remote = repo.remote(name, url);
                todo!()
            }
            None => todo!(),
        }
    }
}
