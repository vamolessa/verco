use std::path::PathBuf;

use crate::backend::{
    Backend, BackendResult, BranchEntry, FileStatus, LogEntry, Process,
    RevisionEntry, RevisionInfo, StatusInfo, TagEntry,
};

pub struct Hg;

impl Hg {
    pub fn try_new() -> Option<(PathBuf, Self)> {
        let output = Process::spawn("hg", &["root"]).ok()?.wait().ok()?;

        let dir = output.lines().next()?;
        let mut root = PathBuf::new();
        root.push(dir);
        Some((root, Self {}))
    }
}

impl Backend for Hg {
    fn status(&self) -> BackendResult<StatusInfo> {
        let header = Process::spawn("hg", &["summary"])?;
        let output = Process::spawn("hg", &["status"])?;

        let header = header.wait()?;
        let output = output.wait()?;

        let entries = output
            .lines()
            .map(str::trim)
            .filter(|e| e.len() > 1)
            .map(|e| {
                let (status, filename) = e.split_at(1);
                RevisionEntry {
                    name: filename.trim().into(),
                    status: parse_file_status(status.trim()),
                }
            })
            .collect();

        Ok(StatusInfo { header, entries })
    }

    fn commit(
        &self,
        message: &str,
        entries: &[RevisionEntry],
    ) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("hg", &["commit", "--addremove", "-m", message])?
                .wait()?;
        } else {
            let mut args = Vec::new();
            args.push("remove");
            for entry in entries {
                if let FileStatus::Missing | FileStatus::Deleted = entry.status
                {
                    args.push(&entry.name);
                }
            }
            let remove = Process::spawn("hg", &args)?;

            args.clear();
            args.push("add");
            for entry in entries {
                if let FileStatus::Untracked = entry.status {
                    args.push(&entry.name);
                }
            }
            let add = Process::spawn("hg", &args)?;

            remove.wait()?;
            add.wait()?;

            Process::spawn("git", &["commit", "-m", message])?.wait()?;
        }

        Ok(())
    }

    fn discard(&self, entries: &[RevisionEntry]) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("hg", &["revert", "-C", "--all"])?.wait()?;
            Process::spawn("hg", &["purge"])?.wait()?;
        } else {
            let mut args = Vec::new();
            args.push("purge");
            for entry in entries {
                if let FileStatus::Untracked = entry.status {
                    args.push(&entry.name);
                }
            }
            let purge = Process::spawn("hg", &args)?;

            args.clear();
            args.push("revert");
            args.push("-C");
            for entry in entries {
                if !matches!(entry.status, FileStatus::Untracked) {
                    args.push(&entry.name);
                }
            }
            let revert = Process::spawn("hg", &args)?;

            purge.wait()?;
            revert.wait()?;
        }

        Ok(())
    }

    // TODO: stopped here
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

    fn resolve_taking_local(
        &self,
        entries: &[RevisionEntry],
    ) -> BackendResult<()> {
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

    fn resolve_taking_other(
        &self,
        entries: &[RevisionEntry],
    ) -> BackendResult<()> {
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
        let message = Process::spawn("git", &["show", "-s", "--format=%B"])?;
        let output = Process::spawn(
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

        let output = output.wait()?;
        let mut splits = output.split('\0').map(str::trim);

        let mut entries = Vec::new();
        loop {
            let status = match splits.next() {
                Some(status) => parse_file_status(status),
                None => break,
            };
            let name = match splits.next() {
                Some(name) => name.into(),
                None => break,
            };

            entries.push(RevisionEntry { name, status });
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
        Process::spawn(
            "git",
            &["push", "--set-upstream", remote.trim(), name],
        )?
        .wait()?;
        Ok(())
    }

    fn delete_branch(&self, name: &str) -> BackendResult<()> {
        let remote = Process::spawn("git", &["remote"])?.wait()?;
        Process::spawn("git", &["branch", "--delete", name])?.wait()?;
        Process::spawn("git", &["push", "--delete", remote.trim(), name])?
            .wait()?;
        Ok(())
    }

    fn tags(&self) -> BackendResult<Vec<TagEntry>> {
        let entries = Process::spawn(
            "git",
            &["tag", "--list", "--format=%(refname:short)"],
        )?
        .wait()?
        .lines()
        .map(|l| TagEntry {
            name: l.trim().into(),
        })
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
        Process::spawn("git", &["push", "--delete", remote.trim(), name])?
            .wait()?;
        Ok(())
    }
}

fn parse_file_status(s: &str) -> FileStatus {
    match s {
        "?" => FileStatus::Untracked,
        "M" => FileStatus::Modified,
        "A" => FileStatus::Added,
        "R" => FileStatus::Deleted,
        "!" => FileStatus::Missing,
        "I" => FileStatus::Ignored,
        "C" => FileStatus::Clean,
        _ => FileStatus::Copied,
    }
}
