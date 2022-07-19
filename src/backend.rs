use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
};

use crate::mode::{FilterEntry, FuzzyMatcher};

pub mod git;
pub mod hg;
pub mod plastic;

pub type BackendResult<T> = std::result::Result<T, String>;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Copied,
    Unmerged,
    Missing,
    Ignored,
    Clean,
    Unknown(String),
}
impl FileStatus {
    pub const fn max_len() -> usize {
        9
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Modified => "modified",
            Self::Added => "added",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
            Self::Untracked => "untracked",
            Self::Copied => "copied",
            Self::Unmerged => "unmerged",
            Self::Missing => "missing",
            Self::Ignored => "ignored",
            Self::Clean => "clean",
            Self::Unknown(status) => {
                if status.len() > Self::max_len() {
                    &status[..Self::max_len()]
                } else {
                    status
                }
            }
        }
    }
}

pub struct StatusInfo {
    pub header: String,
    pub entries: Vec<RevisionEntry>,
}

pub struct RevisionInfo {
    pub message: String,
    pub entries: Vec<RevisionEntry>,
}

#[derive(Clone)]
pub struct RevisionEntry {
    pub selected: bool,
    pub name: String,
    pub status: FileStatus,
}
impl RevisionEntry {
    pub fn new(name: String, status: FileStatus) -> Self {
        Self {
            selected: false,
            name,
            status,
        }
    }
}
impl FilterEntry for RevisionEntry {
    fn fuzzy_matches(&self, matcher: &mut FuzzyMatcher, pattern: &str) -> bool {
        matcher.fuzzy_matches(&self.name, pattern)
    }
}

pub struct LogEntry {
    pub graph: String,
    pub hash: String,
    pub date: String,
    pub author: String,
    pub refs: String,
    pub message: String,
}
impl FilterEntry for LogEntry {
    fn fuzzy_matches(&self, matcher: &mut FuzzyMatcher, pattern: &str) -> bool {
        matcher.fuzzy_matches(&self.message, pattern)
            || matcher.fuzzy_matches(&self.refs, pattern)
            || matcher.fuzzy_matches(&self.author, pattern)
            || matcher.fuzzy_matches(&self.date, pattern)
            || matcher.fuzzy_matches(&self.hash, pattern)
    }
}

#[derive(Clone)]
pub struct BranchEntry {
    pub name: String,
    pub upstream_name: String,
    pub tracking_status: String,
    pub checked_out: bool,
}
impl FilterEntry for BranchEntry {
    fn fuzzy_matches(&self, matcher: &mut FuzzyMatcher, pattern: &str) -> bool {
        matcher.fuzzy_matches(&self.name, pattern)
    }
}

#[derive(Clone)]
pub struct TagEntry {
    pub name: String,
}
impl FilterEntry for TagEntry {
    fn fuzzy_matches(&self, matcher: &mut FuzzyMatcher, pattern: &str) -> bool {
        matcher.fuzzy_matches(&self.name, pattern)
    }
}

pub trait Backend: 'static + Send + Sync {
    fn status(&self) -> BackendResult<StatusInfo>;
    fn commit(&self, message: &str, entries: &[RevisionEntry]) -> BackendResult<()>;
    fn discard(&self, entries: &[RevisionEntry]) -> BackendResult<()>;
    fn diff(&self, revision: Option<&str>, entries: &[RevisionEntry]) -> BackendResult<String>;
    fn resolve_taking_ours(&self, entries: &[RevisionEntry]) -> BackendResult<()>;
    fn resolve_taking_theirs(&self, entries: &[RevisionEntry]) -> BackendResult<()>;

    fn log(&self, start: usize, len: usize) -> BackendResult<(usize, Vec<LogEntry>)>;
    fn checkout_revision(&self, revision: &str) -> BackendResult<()>;
    fn checkout_branch(&self, branch: &BranchEntry) -> BackendResult<()>;
    fn checkout_tag(&self, tag: &TagEntry) -> BackendResult<()>;
    fn merge_branch(&self, branch: &BranchEntry) -> BackendResult<()>;
    fn fetch(&self) -> BackendResult<()>;
    fn fetch_branch(&self, branch: &BranchEntry) -> BackendResult<()>;
    fn pull(&self) -> BackendResult<()>;
    fn push(&self) -> BackendResult<()>;

    fn revision_details(&self, revision: &str) -> BackendResult<RevisionInfo>;

    fn branches(&self) -> BackendResult<Vec<BranchEntry>>;
    fn new_branch(&self, name: &str) -> BackendResult<()>;
    fn delete_branch(&self, branch: &BranchEntry) -> BackendResult<()>;

    fn tags(&self) -> BackendResult<Vec<TagEntry>>;
    fn new_tag(&self, name: &str) -> BackendResult<()>;
    fn delete_tag(&self, tag: &TagEntry) -> BackendResult<()>;
}

pub struct Process(Child);
impl Process {
    pub fn spawn(command_name: &str, args: &[&str]) -> BackendResult<Self> {
        let mut command = Command::new(command_name);
        command.args(args);
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        match command.spawn() {
            Ok(child) => Ok(Self(child)),
            Err(error) => Err(format!(
                "could not spawn process '{}': {}",
                command_name, error
            )),
        }
    }

    pub fn wait(self) -> BackendResult<String> {
        let output = match self.0.wait_with_output() {
            Ok(output) => output,
            Err(error) => return Err(format!("could not wait for process: {}", error)),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            Ok(stdout.into())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut error = String::new();
            error.push_str(&stdout);
            error.push('\n');
            error.push_str(&stderr);
            Err(error)
        }
    }
}

pub fn backend_from_current_repository() -> Option<(PathBuf, Arc<dyn Backend>)> {
    if let Some((root, git)) = git::Git::try_new() {
        Some((root, Arc::new(git)))
    } else if let Some((root, hg)) = hg::Hg::try_new() {
        Some((root, Arc::new(hg)))
    } else if let Some((root, plastic)) = plastic::Plastic::try_new() {
        Some((root, Arc::new(plastic)))
    } else {
        None
    }
}
