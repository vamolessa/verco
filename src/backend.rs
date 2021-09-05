use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
};

use crate::ui::{Drawer, SelectEntryDraw};

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
    pub name: String,
    pub status: FileStatus,
}

pub struct LogEntry {
    pub graph: String,
    pub hash: String,
    pub date: String,
    pub author: String,
    pub refs: String,
    pub message: String,
}

pub struct BranchEntry {
    pub name: String,
    pub checked_out: bool,
}

pub struct TagEntry {
    pub name: String,
}

#[derive(Clone)]
pub struct SelectableRevisionEntry {
    pub selected: bool,
    pub name: String,
    pub status: FileStatus,
}
impl From<RevisionEntry> for SelectableRevisionEntry {
    fn from(other: RevisionEntry) -> Self {
        Self {
            selected: false,
            name: other.name,
            status: other.status,
        }
    }
}
impl SelectEntryDraw for SelectableRevisionEntry {
    fn draw(&self, drawer: &mut Drawer, _: bool, _: bool) -> usize {
        const NAME_TOO_LONG_PREFIX: &str = "...";

        let name_available_width = (drawer.viewport_size.0 as usize)
            .saturating_sub(2 + 1 + FileStatus::max_len() + 1 + 1 + NAME_TOO_LONG_PREFIX.len());

        let (name_prefix, trimmed_name) =
            match self.name.char_indices().nth_back(name_available_width) {
                Some((i, _)) => (NAME_TOO_LONG_PREFIX, &self.name[i..]),
                None => ("", &self.name[..]),
            };

        let selected_text = if self.selected { '+' } else { ' ' };
        drawer.fmt(format_args!(
            "{} [{:>width$}] {}{}",
            selected_text,
            self.status.as_str(),
            name_prefix,
            trimmed_name,
            width = FileStatus::max_len(),
        ));

        1
    }
}

pub trait Backend: 'static + Send + Sync {
    fn status(&self) -> BackendResult<StatusInfo>;
    fn commit(&self, message: &str, entries: &[RevisionEntry]) -> BackendResult<()>;
    fn discard(&self, entries: &[RevisionEntry]) -> BackendResult<()>;
    fn diff(&self, revision: Option<&str>, entries: &[RevisionEntry]) -> BackendResult<String>;
    fn resolve_taking_ours(&self, entries: &[RevisionEntry]) -> BackendResult<()>;
    fn resolve_taking_theirs(&self, entries: &[RevisionEntry]) -> BackendResult<()>;

    fn log(&self, start: usize, len: usize) -> BackendResult<Vec<LogEntry>>;
    fn checkout(&self, revision: &str) -> BackendResult<()>;
    fn merge(&self, revision: &str) -> BackendResult<()>;
    fn fetch(&self) -> BackendResult<()>;
    fn pull(&self) -> BackendResult<()>;
    fn push(&self) -> BackendResult<()>;

    fn revision_details(&self, revision: &str) -> BackendResult<RevisionInfo>;

    fn branches(&self) -> BackendResult<Vec<BranchEntry>>;
    fn new_branch(&self, name: &str) -> BackendResult<()>;
    fn delete_branch(&self, name: &str) -> BackendResult<()>;

    fn tags(&self) -> BackendResult<Vec<TagEntry>>;
    fn new_tag(&self, name: &str) -> BackendResult<()>;
    fn delete_tag(&self, name: &str) -> BackendResult<()>;
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
