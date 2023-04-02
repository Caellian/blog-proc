use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use git2::{Remote, Repository};

use crate::{blog::Blog, error::BlogError, util::MinMax};

pub struct FileHistory {
    edits: Vec<DateTime<Utc>>,
}

impl FileHistory {
    pub fn new() -> Self {
        FileHistory {
            edits: Vec::with_capacity(16),
        }
    }
}

impl FromIterator<DateTime<Utc>> for FileHistory {
    fn from_iter<T: IntoIterator<Item = DateTime<Utc>>>(iter: T) -> Self {
        FileHistory {
            edits: iter.into_iter().collect(),
        }
    }
}

impl Into<MinMax<DateTime<Utc>>> for FileHistory {
    fn into(self) -> MinMax<DateTime<Utc>> {
        MinMax::from(self.edits.into_iter())
    }
}

pub trait ExtRepository {
    fn remote_list(&self) -> Vec<Remote>;
    fn has_remote(&self, url: impl AsRef<str>) -> bool;
    fn remote_name(&self, url: impl AsRef<str>) -> Option<String>;
    fn file_history(&self, file: impl AsRef<Path>) -> FileHistory;
}

impl ExtRepository for Repository {
    fn remote_list(&self) -> Vec<Remote> {
        // get a list of remotes from remote names
        self.remotes()
            .map(|remotes| {
                remotes
                    .iter()
                    .filter_map(|it| it)
                    .filter_map(|remote| self.find_remote(remote).ok().to_owned())
                    .collect()
            })
            .unwrap_or_else(|_| Vec::new())
    }

    fn has_remote(&self, url: impl AsRef<str>) -> bool {
        self.remote_list()
            .into_iter()
            .any(|r| r.url().map(|u| u == url.as_ref()) == Some(true))
    }

    fn remote_name(&self, url: impl AsRef<str>) -> Option<String> {
        self.remote_list()
            .into_iter()
            .find(|r| r.url().map(|u| u == url.as_ref()) == Some(true))
            .map(|r| r.name().map(|n| n.to_string()))
            .flatten()
    }

    fn file_history(&self, file: impl AsRef<Path>) -> FileHistory {
        let now = Utc::now();

        self.revwalk()
            .unwrap()
            .filter_map(|it| {
                if let Ok(oid) = it {
                    self.find_object(oid, Some(git2::ObjectType::Commit))
                        .ok()
                        .map(|it| it.as_commit().unwrap())
                } else {
                    None
                }
            })
            .filter_map(|commit| {
                if commit
                    .tree()
                    .map(|tree| tree.get_path(file.as_ref()).is_ok())
                    .unwrap_or_default()
                {
                    Some(commit.time().to_chrono())
                } else {
                    None
                }
            })
            .collect()
    }
}

pub trait ToChronoExt {
    fn to_chrono(self) -> DateTime<Utc>;
}

impl ToChronoExt for git2::Time {
    fn to_chrono(self) -> DateTime<Utc> {
        let mut result = NaiveDateTime::from_timestamp(self.seconds(), 0)
            .and_local_timezone(Utc)
            .unwrap();
        result.checked_sub_signed(Duration::minutes(self.offset_minutes() as i64));
        result
    }
}

pub fn clone_blog(
    repo_url: impl AsRef<str>,
    work_dir: impl Into<PathBuf>,
) -> Result<Blog, BlogError> {
    let root: PathBuf = work_dir.into();

    let repo = if root.exists() {
        log::info!("Found an existing repository.");
        let repo = Repository::open(&root)?;

        if !repo.has_remote(repo_url.as_ref()) {
            return Err(BlogError::RepoMismatch {
                expected: repo_url.as_ref().to_string(),
                existing: repo.remote_name(repo_url.as_ref()).unwrap_or_default(),
            });
        }

        Some(repo)
    } else {
        log::info!(
            "Cloning blog repo ({}) into {} ...",
            repo_url.as_ref(),
            root.display()
        );

        Some(Repository::clone_recurse(repo_url.as_ref(), &root)?)
    };

    Ok(Blog {
        source_dir: root,
        repo,
        ..Default::default()
    })
}
