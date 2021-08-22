use std::path::PathBuf;

use crate::backend::{
    Backend, BackendResult, FileStatus, LogEntry, Process, RevisionEntry,
    StatusInfo,
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
                RevisionEntry {
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
        entries: &[RevisionEntry],
    ) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("git", &["add", "--all"])?.wait()?;
        } else {
            for entry in entries {
                Process::spawn("git", &["add", "--", &entry.name])?.wait()?;
            }
        }

        Process::spawn("git", &["commit", "-m", message])?.wait()?;
        Ok(())
    }

    fn discard(&self, entries: &[RevisionEntry]) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("git", &["reset", "--hard"])?.wait()?;
            Process::spawn("git", &["clean", "-d", "--force"])?.wait()?;
            Ok(())
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

            for process in processes {
                process.wait()?;
            }

            Ok(())
        }
    }

    fn diff(
        &self,
        revision: Option<&str>,
        entries: &[RevisionEntry],
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

    fn log(&self, start: usize, len: usize) -> BackendResult<Vec<LogEntry>> {
        let start = start.to_string();
        let len = len.to_string();
        let template = "--format=format:%x00%h%x00%as%x00%aN%x00%D%x00%s";
        let output = Process::spawn(
            "git",
            &[
                "log",
                "--all",
                "--decorate",
                "--oneline",
                "--graph",
                "--skip",
                &start,
                "--max-count",
                &len,
                template,
            ],
        )?
        .wait()?;

        let mut entries = Vec::new();
        for line in output.lines() {
            let mut splits = line.splitn(6, '\0');

            let graph = splits.next().unwrap_or("").into();
            let hash = splits.next().unwrap_or("").into();
            let date = splits.next().unwrap_or("").into();
            let author = splits.next().unwrap_or("").into();
            let refs = splits.next().unwrap_or("").into();
            let message = splits.next().unwrap_or("").into();

            entries.push(LogEntry {
                graph,
                hash,
                date,
                author,
                refs,
                message,
            });
        }

        Ok(entries)
    }

    fn checkout(&self, revision: &str) -> BackendResult<()> {
        Process::spawn("git", &["checkout", revision])?.wait()?;
        Ok(())
    }

    fn fetch(&self) -> BackendResult<()> {
        Process::spawn("git", &["fetch", "--all"])?.wait()?;
        Ok(())
    }

    fn pull(&self) -> BackendResult<()> {
        Process::spawn("git", &["pull", "--all"])?.wait()?;
        Ok(())
    }

    fn push(&self) -> BackendResult<()> {
        Process::spawn("git", &["push"])?.wait()?;
        Ok(())
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

