use std::path::{Path, PathBuf};

use crate::backend::{
    Backend, BackendResult, BranchEntry, FileStatus, LogEntry, Process,
    RevisionEntry, RevisionInfo, StatusInfo, TagEntry,
};

pub struct Plastic;

impl Plastic {
    pub fn try_new() -> Option<(PathBuf, Self)> {
        let output = Process::spawn(
            "cm",
            &["getworkspacefrompath", "--format={wkpath}", "."],
        )
        .ok()?
        .wait()
        .ok()?;

        let root = Path::new(output.trim()).into();
        Some((root, Self))
    }
}

impl Backend for Plastic {
    fn status(&self) -> BackendResult<StatusInfo> {
        let header = Process::spawn("cm", &["status", "--header"])?;
        let output = Process::spawn(
            "cm",
            &[
                "status",
                "--short",
                "--nomergesinfo",
                "--machinereadable",
                "--fieldseparator=;",
            ],
        )?;

        let header = header.wait()?.trim().into();
        let output = output.wait()?;

        let mut entries = Vec::new();

        for line in output.lines() {
            let mut splits = line.split(';');
            let status = splits.next().unwrap_or("").trim();
            let status = parse_file_status(status);
            let name = splits.next().unwrap_or("").trim().into();
            splits.next();
            let _mergeinfo = splits.next().unwrap_or("").trim();

            entries.push(RevisionEntry { name, status });
        }

        Ok(StatusInfo { header, entries })
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
            let clean = Process::spawn("git", &args)?;

            args.clear();
            args.push("rm");
            args.push("--force");
            args.push("--");
            for entry in entries {
                if let FileStatus::Added = entry.status {
                    args.push(&entry.name);
                }
            }
            let rm = Process::spawn("git", &args)?;

            args.clear();
            args.push("checkout");
            args.push("--");
            for entry in entries {
                if !matches!(
                    entry.status,
                    FileStatus::Untracked | FileStatus::Added,
                ) {
                    args.push(&entry.name);
                }
            }
            let checkout = Process::spawn("git", &args)?;

            clean.wait()?;
            rm.wait()?;
            checkout.wait()?;
        }

        Ok(())
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
        "CH" => FileStatus::Modified,
        "LD" => FileStatus::Deleted,
        "PR" => FileStatus::Untracked,
        _ => FileStatus::Other(s.into()),
    }

    /*
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
    */
}

