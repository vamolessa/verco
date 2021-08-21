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
        let output =
            Process::spawn("git", &["status", "--branch", "--null"])?.wait()?;
        let mut splits = output.trim().split('\0').map(str::trim);

        let header = splits.next().unwrap_or("").into();
        let entries = splits
            .filter(|e| e.len() >= 2)
            .map(|e| {
                let (status, filename) = e.split_at(2);
                StatusEntry {
                    name: filename.trim().into(),
                    status: parse_file_status(status.trim()),
                }
            })
            .collect();

        let mut info = StatusInfo { header, entries };
        info.entries.sort_unstable_by_key(|e| e.status as usize);

        Ok(info)
    }

    fn commit(
        &self,
        message: &str,
        entries: &[StatusEntry],
    ) -> BackendResult<String> {
        if entries.is_empty() {
            Process::spawn("git", &["add", "--all"])?.wait()?;
        } else {
            for entry in entries {
                Process::spawn("git", &["add", "--", &entry.name])?.wait()?;
            }
        }

        let output =
            Process::spawn("git", &["commit", "-m", message])?.wait()?;
        Ok(output)
    }

    fn discard(&self, entries: &[StatusEntry]) -> BackendResult<String> {
        if entries.is_empty() {
            let mut output = String::new();
            output.push_str(
                &Process::spawn("git", &["reset", "--hard"])?.wait()?,
            );
            output.push_str(
                &Process::spawn("git", &["clean", "-d", "--force"])?.wait()?,
            );
            Ok(output)
        } else {
            let mut processes = Vec::new();
            for entry in entries {
                match entry.status {
                    FileStatus::Untracked => processes.push(Process::spawn(
                        "git",
                        &["clean", "--force", "--", &entry.name],
                    )?),
                    FileStatus::Added => processes.push(Process::spawn(
                        "git",
                        &["rm", "--force", "--", &entry.name],
                    )?),
                    _ => processes.push(Process::spawn(
                        "git",
                        &["checkout", "--", &entry.name],
                    )?),
                }
            }

            let mut output = String::new();
            for process in processes {
                output.push_str(&process.wait()?);
            }

            Ok(output)
        }
    }

    fn diff(
        &self,
        revision: Option<&str>,
        entries: &[StatusEntry],
    ) -> BackendResult<String> {
        match revision {
            Some(revision) => {
                let parent = format!("{}^@", revision);
                if entries.is_empty() {
                    Process::spawn(
                        "git",
                        &["diff", "--color", &parent, revision],
                    )?
                    .wait()
                } else {
                    let mut args = Vec::new();
                    args.push("diff");
                    args.push("--color");
                    args.push(&parent);
                    args.push(revision);
                    args.push("--");
                    for entry in entries {
                        args.push(&entry.name);
                    }

                    Process::spawn("git", &args)?.wait()
                }
            }
            None => {
                if entries.is_empty() {
                    Process::spawn("git", &["diff", "--color"])?.wait()
                } else {
                    let mut args = Vec::new();
                    args.push("diff");
                    args.push("--color");
                    args.push("--");
                    for entry in entries {
                        args.push(&entry.name);
                    }
                    Process::spawn("git", &args)?.wait()
                }
            }
        }
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

