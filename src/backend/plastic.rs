use std::{
    fs,
    path::{Path, PathBuf},
};

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
                "--fieldseparator=\x1f",
            ],
        )?;

        let header = header.wait()?.trim().into();
        let output = output.wait()?;

        let mut entries = Vec::new();

        for line in output.lines() {
            let mut splits = line.split('\x1f');
            let status = splits.next().unwrap_or("");
            let status = parse_file_status(status);
            let name = splits.next().unwrap_or("").into();
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
            let untracked = Process::spawn(
                "cm",
                &[
                    "status",
                    "--short",
                    "--nomergesinfo",
                    "--private",
                    "--machinereadable",
                    "--fieldseparator=\x1f",
                ],
            )?
            .wait()?;

            let mut args = Vec::new();
            args.push("add");
            for line in untracked.lines() {
                if let Some(name) = line.split('\x1f').nth(1) {
                    args.push(name);
                }
            }
            if args.len() > 1 {
                Process::spawn("cm", &args)?.wait()?;
            }

            Process::spawn("cm", &["checkin", "--all"])?.wait()?;
        } else {
            let mut args = Vec::new();
            args.push("add");
            for entry in entries {
                if let FileStatus::Untracked = entry.status {
                    args.push(&entry.name);
                }
            }
            if args.len() > 1 {
                Process::spawn("cm", &args)?.wait()?;
            }

            args.clear();
            args.push("checkin");
            for entry in entries {
                args.push(&entry.name);
            }
            Process::spawn("cm", &args)?.wait()?;
            Process::spawn("cm", &["commit", "-m", message])?.wait()?;
        }

        Ok(())
    }

    fn discard(&self, entries: &[RevisionEntry]) -> BackendResult<()> {
        fn delete_file(name: &str) -> BackendResult<()> {
            fs::remove_file(name).map_err(|e| e.to_string())
        }

        if entries.is_empty() {
            Process::spawn("cm", &["undo", ".", "-r"])?.wait()?;
            let untracked = Process::spawn(
                "cm",
                &[
                    "status",
                    "--short",
                    "--nomergesinfo",
                    "--private",
                    "--machinereadable",
                    "--fieldseparator=\x1f",
                ],
            )?
            .wait()?;

            for line in untracked.lines() {
                if let Some(name) = line.split('\x1f').nth(1) {
                    delete_file(name)?;
                }
            }
        } else {
            let mut args = Vec::new();
            args.clear();
            args.push("undo");
            for entry in entries {
                match entry.status {
                    FileStatus::Untracked => delete_file(&entry.name)?,
                    _ => args.push(&entry.name),
                }
            }
            if args.len() > 1 {
                Process::spawn("cm", &args)?.wait()?;
            }
        }

        Ok(())
    }

    // TODO: implement diff somehow
    fn diff(
        &self,
        revision: Option<&str>,
        entries: &[RevisionEntry],
    ) -> BackendResult<String> {
        match revision {
            Some(revision) => {
                if entries.is_empty() {
                    //
                } else {
                    //
                }
            }
            None => {
                if entries.is_empty() {
                    //
                } else {
                    //
                }
            }
        }

        Err("diff is not yet implemented".into())
    }

    // TODO
    fn resolve_taking_ours(
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

    // TODO
    fn resolve_taking_theirs(
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

    fn log(&self, skip: usize, len: usize) -> BackendResult<Vec<LogEntry>> {
        let output = Process::spawn(
            "cm",
            &[
                "find",
                "changeset",
                "--nototal",
                "--format={changesetid}\x1f{date}\x1f{owner}\x1f{branch}\x1f{comment}\x1e",
            ],
        )?
        .wait()?;

        let mut entries = Vec::new();
        for record in output.split('\x1e').rev().skip(skip + 1).take(len) {
            let mut splits = record.splitn(5, '\x1f');

            let graph = String::new();
            let hash = splits.next().unwrap_or("").trim().into();
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
        Process::spawn("cm", &["switch", revision])?.wait()?;
        Ok(())
    }

    // TODO
    fn merge(&self, revision: &str) -> BackendResult<()> {
        Process::spawn("git", &["merge", revision])?.wait()?;
        Ok(())
    }

    fn fetch(&self) -> BackendResult<()> {
        Ok(())
    }

    fn pull(&self) -> BackendResult<()> {
        Process::spawn("cm", &["update"])?.wait()?;
        Ok(())
    }

    fn push(&self) -> BackendResult<()> {
        Ok(())
    }

    // TODO
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
        let current_branch = Process::spawn("cm", &["status", "--header"])?;
        let entries = Process::spawn(
            "cm",
            &["find", "branch", "--nototal", "--format={name}"],
        )?;

        let current_branch = current_branch.wait()?;
        let current_branch = current_branch.split('@').next().unwrap_or("");

        let entries = entries
            .wait()?
            .lines()
            .map(|name| {
                let name = name.into();
                let checked_out = name == current_branch;
                BranchEntry { name, checked_out }
            })
            .collect();

        Ok(entries)
    }

    fn new_branch(&self, name: &str) -> BackendResult<()> {
        Process::spawn("cm", &["branch", "create", name])?.wait()?;
        Ok(())
    }

    fn delete_branch(&self, name: &str) -> BackendResult<()> {
        Process::spawn("cm", &["branch", "delete", name])?.wait()?;
        Ok(())
    }

    fn tags(&self) -> BackendResult<Vec<TagEntry>> {
        let entries = Process::spawn(
            "cm",
            &["find", "label", "--nototal", "--format={name}"],
        )?
        .wait()?
        .lines()
        .map(|l| TagEntry { name: l.into() })
        .collect();
        Ok(entries)
    }

    fn new_tag(&self, name: &str) -> BackendResult<()> {
        Process::spawn("cm", &["label", "create", name])?.wait()?;
        Ok(())
    }

    fn delete_tag(&self, name: &str) -> BackendResult<()> {
        Process::spawn("cm", &["label", "delete", name])?.wait()?;
        Ok(())
    }
}

fn parse_file_status(s: &str) -> FileStatus {
    match s {
        "CH" => FileStatus::Modified,
        "CO" => FileStatus::Added,
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

