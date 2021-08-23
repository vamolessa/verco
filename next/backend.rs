use std::{
    fmt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
};

pub mod git;

pub type BackendResult<T> = std::result::Result<T, String>;

#[derive(Clone, Copy)]
pub enum FileStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Copied,
    Unmerged,
    //Missing,
    //Ignored,
    //Clean,
}
impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Unmodified => f.write_str("unmodified"),
            Self::Modified => f.write_str("modified"),
            Self::Added => f.write_str("added"),
            Self::Deleted => f.write_str("deleted"),
            Self::Renamed => f.write_str("renamed"),
            Self::Untracked => f.write_str("untracked"),
            Self::Copied => f.write_str("copied"),
            Self::Unmerged => f.write_str("unmerged"),
            //Self::Missing => f.write_str("missing"),
            //Self::Ignored => f.write_str("ignored"),
            //Self::Clean => f.write_str("clean"),
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

pub trait Backend: 'static + Send + Sync {
    fn name(&self) -> &str;

    fn status(&self) -> BackendResult<StatusInfo>;
    fn commit(
        &self,
        message: &str,
        entries: &[RevisionEntry],
    ) -> BackendResult<()>;
    fn discard(&self, entries: &[RevisionEntry]) -> BackendResult<()>;
    fn diff(
        &self,
        revision: Option<&str>,
        entries: &[RevisionEntry],
    ) -> BackendResult<String>;
    fn resolve_taking_local(&self, entries: &[RevisionEntry]) -> BackendResult<()>;
    fn resolve_taking_other(&self, entries: &[RevisionEntry]) -> BackendResult<()>;

    fn log(&self, start: usize, len: usize) -> BackendResult<Vec<LogEntry>>;
    fn checkout(&self, revision: &str) -> BackendResult<()>;
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
            Err(error) => {
                return Err(format!("could not wait for process: {}", error))
            }
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

pub fn backend_from_current_repository() -> Option<(PathBuf, Arc<dyn Backend>)>
{
    if let Some((root, git)) = git::Git::try_new() {
        Some((root, Arc::new(git)))
    } else {
        None
    }
}

