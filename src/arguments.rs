use std::{ffi::OsStr, ops::Deref, path::PathBuf, str::FromStr};

use chrono::NaiveDateTime;
use clap::{Parser, Subcommand};
use regex::Regex;

use crate::error::{BlogError, UserError};

#[derive(Debug, Clone)]
pub struct RepoUrl(String);

impl Deref for RepoUrl {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

lazy_static::lazy_static! {
    pub static ref REPO_URL_PATTERN: Regex = Regex::new(r"https?://\w(\.\w)+(/[\w_-])+\.git").unwrap();
}

impl RepoUrl {
    pub fn new(url: impl AsRef<str>) -> Result<RepoUrl, BlogError> {
        let url = url.as_ref().to_string();

        if REPO_URL_PATTERN.is_match(&url) {
            Ok(RepoUrl(url))
        } else {
            Err(UserError::InvalidRepoUrl(url).into())
        }
    }

    pub unsafe fn new_unchecked(url: impl AsRef<str>) -> RepoUrl {
        RepoUrl(url.as_ref().to_string())
    }
}

impl AsRef<str> for RepoUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ToString for RepoUrl {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl FromStr for RepoUrl {
    type Err = BlogError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RepoUrl::new(s)
    }
}

impl<'a> From<&'a str> for RepoUrl {
    fn from(os_str: &'a str) -> Self {
        RepoUrl::new(os_str).expect("invalid repo url")
    }
}

pub struct SeparatorList<L, S> {
    pub list: L,
    pub sep: S,
}

impl<'a, S: AsRef<str>> SeparatorList<S, char> {
    pub fn list(&self) -> Vec<&str> {
        self.list.as_ref().split(self.sep).collect()
    }
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Directory used for processing posts
    #[arg(short = 'w', long = "work-dir", default_value = ".")]
    pub working_dir: PathBuf,

    /// Output directory for generated JSON files
    #[arg(short = 'o', long = "output-dir", default_value = "./.blog-meta")]
    pub target_dir: PathBuf,

    /// Print output to stdout instead of file
    #[arg(long = "stdout", default_value_t = false)]
    pub print_output: bool,

    /// Action to perform
    #[command(subcommand)]
    pub verb: Verb,
}

#[derive(Debug, Subcommand)]
pub enum Verb {
    /// Clones remote blog repository to local path
    Clone(GitSource),
    /// Syncronizes local and upstream changes
    Pull,
    /// Update file index
    Index,
    /// Watch files to update indices and generated files on change
    Watch,
    /// Builds metadata files and pages
    Build,
    /// Print a list of posts for query
    Posts, // PostQuery
    /// Mark post published and push it
    Publish,
}

#[derive(Debug, Parser)]
pub struct GitSource {
    /// Repository to sync `working_direcotory` to
    #[arg(short = 'r', long = "repo")]
    pub repo: RepoUrl,

    /// Repository branch to clone or sync to
    #[arg(short = 'b', long = "branch", default_value = "master")]
    pub repo_branch: String,
}
