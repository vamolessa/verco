use std::path::PathBuf;

use crate::backend::{
    Backend, BackendResult, FileStatus, Process, StatusEntry, StatusInfo,
};

pub struct Git;

impl Git {
    pub fn try_new() -> Option<(PathBuf, Self)> {
        let output = Process::spawn("git", &["rev-parse", "--show-toplevel"])
            .ok()?
            .wait()
            .ok()?;

        let dir = output.lines().next()?;
        let mut root = PathBuf::new();
        root.push(dir);
        Some((root, Self {}))
    }
}

impl Backend for Git {
    fn name(&self) -> &str {
        "git"
    }

    fn status(&self) -> BackendResult<StatusInfo> {
        let entries = Process::spawn("git", &["status", "-z"])?
            .wait()?
            .trim()
            .split('\0')
            .map(str::trim)
            .filter(|e| e.len() >= 2)
            .map(|e| {
                let (status, filename) = e.split_at(2);
                StatusEntry {
                    name: filename.trim().into(),
                    status: parse_file_status(status.trim()),
                }
            })
            .collect();

        let mut info = StatusInfo {
            header: String::new(),
            entries,
        };
        info.entries.sort_unstable_by_key(|e| e.status as usize);

        Ok(info)
    }

    fn commit(&self, message: &str, files: &[String]) -> BackendResult<String> {
        if files.is_empty() {
            Process::spawn("git", &["add", "--all"])?.wait()?;
        } else {
            for file in files {
                Process::spawn("git", &["add", "--", file])?.wait()?;
            }
        }

        let output =
            Process::spawn("git", &["commit", "-m", message])?.wait()?;
        Ok(output)
    }
}

fn parse_file_status(s: &str) -> FileStatus {
    match s {
        "M" => FileStatus::Modified,
        "A" => FileStatus::Added,
        "D" => FileStatus::Deleted,
        "R" => FileStatus::Renamed,
        "?" => FileStatus::Untracked,
        "C" => FileStatus::Copied,
        "U" => FileStatus::Unmerged,
        _ => FileStatus::Unmodified,
    }
}

