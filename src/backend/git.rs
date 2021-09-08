use std::path::{Path, PathBuf};

use crate::backend::{
    Backend, BackendResult, BranchEntry, FileStatus, LogEntry, Process, RevisionEntry,
    RevisionInfo, StatusInfo, TagEntry,
};

pub struct Git;

impl Git {
    pub fn try_new() -> Option<(PathBuf, Self)> {
        let output = Process::spawn("git", &["rev-parse", "--show-toplevel"])
            .ok()?
            .wait()
            .ok()?;

        let root = Path::new(output.trim()).into();
        Some((root, Self))
    }
}

impl Backend for Git {
    fn status(&self) -> BackendResult<StatusInfo> {
        let output = Process::spawn("git", &["status", "--branch", "--null"])?.wait()?;
        let mut splits = output.split('\0').map(str::trim);

        let header = splits.next().unwrap_or("").into();
        let entries = splits
            .filter(|e| e.len() >= 2)
            .map(|e| {
                let (status, filename) = e.split_at(2);
                RevisionEntry::new(filename.trim().into(), parse_file_status(status))
            })
            .collect();

        Ok(StatusInfo { header, entries })
    }

    fn commit(&self, message: &str, entries: &[RevisionEntry]) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("git", &["add", "--all"])?.wait()?;
        } else {
            let mut args = Vec::new();
            args.push("add");
            args.push("--");
            for entry in entries {
                args.push(&entry.name);
            }

            Process::spawn("git", &args)?.wait()?;
        }

        Process::spawn("git", &["commit", "-m", message])?.wait()?;
        Ok(())
    }

    fn discard(&self, entries: &[RevisionEntry]) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("git", &["reset", "--hard"])?.wait()?;
            Process::spawn("git", &["clean", "-d", "--force"])?.wait()?;
        } else {
            let mut args = Vec::new();
            args.push("clean");
            args.push("--force");
            args.push("--");
            for entry in entries {
                if let FileStatus::Untracked = entry.status {
                    args.push(&entry.name);
                }
            }
            let clean = if args.len() > 3 {
                Some(Process::spawn("git", &args)?)
            } else {
                None
            };

            args.clear();
            args.push("rm");
            args.push("--force");
            args.push("--");
            for entry in entries {
                if let FileStatus::Added = entry.status {
                    args.push(&entry.name);
                }
            }
            let rm = if args.len() > 3 {
                Some(Process::spawn("git", &args)?)
            } else {
                None
            };

            if let Some(clean) = clean {
                clean.wait()?;
            }
            if let Some(rm) = rm {
                rm.wait()?;
            }

            args.clear();
            args.push("checkout");
            args.push("--");
            for entry in entries {
                if !matches!(entry.status, FileStatus::Untracked | FileStatus::Added,) {
                    args.push(&entry.name);
                }
            }
            if args.len() > 2 {
                Process::spawn("git", &args)?.wait()?;
            }
        }

        Ok(())
    }

    fn diff(&self, revision: Option<&str>, entries: &[RevisionEntry]) -> BackendResult<String> {
        match revision {
            Some(revision) => {
                let parent = format!("{}^@", revision);
                if entries.is_empty() {
                    Process::spawn("git", &["diff", &parent, revision])?.wait()
                } else {
                    let mut args = Vec::new();
                    args.push("diff");
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
                    Process::spawn("git", &["diff", "-z"])?.wait()
                } else {
                    let mut args = Vec::new();
                    args.push("diff");
                    args.push("--");
                    for entry in entries {
                        args.push(&entry.name);
                    }
                    Process::spawn("git", &args)?.wait()
                }
            }
        }
    }

    fn resolve_taking_ours(&self, entries: &[RevisionEntry]) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("git", &["checkout", ".", "--ours"])?.wait()?;
        } else {
            if !entries
                .iter()
                .any(|e| matches!(e.status, FileStatus::Unmerged))
            {
                return Ok(());
            }

            let mut args = Vec::new();
            args.push("checkout");
            args.push(".");
            args.push("--ours");
            args.push("--");

            for entry in entries {
                if let FileStatus::Unmerged = entry.status {
                    args.push(&entry.name);
                }
            }

            Process::spawn("git", &args)?.wait()?;
        }

        Ok(())
    }

    fn resolve_taking_theirs(&self, entries: &[RevisionEntry]) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("git", &["checkout", ".", "--theirs"])?.wait()?;
        } else {
            if !entries
                .iter()
                .any(|e| matches!(e.status, FileStatus::Unmerged))
            {
                return Ok(());
            }

            let mut args = Vec::new();
            args.push("checkout");
            args.push(".");
            args.push("--theirs");
            args.push("--");

            for entry in entries {
                if let FileStatus::Unmerged = entry.status {
                    args.push(&entry.name);
                }
            }

            Process::spawn("git", &args)?.wait()?;
        }

        Ok(())
    }

    fn log(&self, skip: usize, len: usize) -> BackendResult<(usize, Vec<LogEntry>)> {
        let skip_text = skip.to_string();
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
                &skip_text,
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

        Ok((skip, entries))
    }

    fn checkout(&self, revision: &str) -> BackendResult<()> {
        Process::spawn("git", &["checkout", revision])?.wait()?;
        Ok(())
    }

    fn merge(&self, revision: &str) -> BackendResult<()> {
        Process::spawn("git", &["merge", revision])?.wait()?;
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

    fn revision_details(&self, revision: &str) -> BackendResult<RevisionInfo> {
        let message = Process::spawn("git", &["show", "-s", "--format=%B", revision])?;
        let changes = Process::spawn(
            "git",
            &[
                "diff-tree",
                "--no-commit-id",
                "--name-status",
                "-r",
                "-z",
                revision,
            ],
        )?;

        let message = message.wait()?.trim().into();

        let changes = changes.wait()?;
        let mut splits = changes.split('\0');

        let mut entries = Vec::new();
        loop {
            let status = match splits.next() {
                Some(status) if !status.is_empty() => parse_file_status(status),
                _ => break,
            };
            let name = match splits.next() {
                Some(name) => name.into(),
                None => break,
            };

            entries.push(RevisionEntry::new(name, status));
        }

        Ok(RevisionInfo { message, entries })
    }

    fn branches(&self) -> BackendResult<Vec<BranchEntry>> {
        let entries = Process::spawn(
            "git",
            &[
                "branch",
                "--list",
                "--all",
                "--format=%(refname:short)%00%(HEAD)",
            ],
        )?
        .wait()?
        .lines()
        .map(|l| {
            let mut splits = l.splitn(2, '\0');
            let name = splits.next().unwrap_or("").into();
            let checked_out = splits.next().unwrap_or("") == "*";
            BranchEntry { name, checked_out }
        })
        .collect();
        Ok(entries)
    }

    fn new_branch(&self, name: &str) -> BackendResult<()> {
        let remote = Process::spawn("git", &["remote"])?.wait()?;
        Process::spawn("git", &["branch", name])?.wait()?;
        Process::spawn("git", &["checkout", name])?.wait()?;
        Process::spawn("git", &["push", "--set-upstream", remote.trim(), name])?.wait()?;
        Ok(())
    }

    fn delete_branch(&self, name: &str) -> BackendResult<()> {
        let remote = Process::spawn("git", &["remote"])?.wait()?;
        Process::spawn("git", &["branch", "--delete", name])?.wait()?;
        Process::spawn("git", &["push", "--delete", remote.trim(), name])?.wait()?;
        Ok(())
    }

    fn tags(&self) -> BackendResult<Vec<TagEntry>> {
        let entries = Process::spawn("git", &["tag", "--list", "--format=%(refname:short)"])?
            .wait()?
            .lines()
            .map(|l| TagEntry { name: l.into() })
            .collect();
        Ok(entries)
    }

    fn new_tag(&self, name: &str) -> BackendResult<()> {
        let remote = Process::spawn("git", &["remote"])?.wait()?;
        Process::spawn("git", &["tag", "--force", name])?.wait()?;
        Process::spawn("git", &["push", remote.trim(), name])?.wait()?;
        Ok(())
    }

    fn delete_tag(&self, name: &str) -> BackendResult<()> {
        let remote = Process::spawn("git", &["remote"])?.wait()?;
        Process::spawn("git", &["tag", "--delete", name])?.wait()?;
        Process::spawn("git", &["push", "--delete", remote.trim(), name])?.wait()?;
        Ok(())
    }
}

fn parse_file_status(s: &str) -> FileStatus {
    match s.chars().next() {
        Some('M') => FileStatus::Modified,
        Some('A') => FileStatus::Added,
        Some('D') => FileStatus::Deleted,
        Some('R') => FileStatus::Renamed,
        Some('?') => FileStatus::Untracked,
        Some('C') => FileStatus::Copied,
        Some('U') => FileStatus::Unmerged,
        Some(' ') => FileStatus::Clean,
        _ => panic!("unknown file status '{}'", s),
    }
}
