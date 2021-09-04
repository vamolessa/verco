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

    fn diff(
        &self,
        revision: Option<&str>,
        entries: &[RevisionEntry],
    ) -> BackendResult<String> {
        let entry = match entries {
            [] => None,
            [entry] => Some(entry),
            _ => return Err("can not diff more than one file at a time".into()),
        };

        match revision {
            Some(revision) => match entry {
                None => {
                    let head =
                        Process::spawn("cm", &["status", "--head"])?.wait()?;
                    let suffix = match head.find('@') {
                        Some(i) => &head[i..],
                        None => return Err("could not parse head".into()),
                    };
                    let changeset_arg =
                        format!("--showchangeset=cs:{}{}", revision, suffix);
                    Process::spawn("plastic", &[&changeset_arg])?.wait()?;
                }
                Some(entry) => {
                    Process::spawn("cm", &["diff", revision, &entry.name])?
                        .wait()?;
                }
            },
            None => match entry {
                None => {
                    return Err(
                        "diff is not implemented for pending changes".into()
                    );
                }
                Some(entry) => {
                    Process::spawn("cm", &["diff", &entry.name])?.wait()?;
                }
            },
        }

        Ok("".into())
    }

    // TODO
    fn resolve_taking_ours(
        &self,
        entries: &[RevisionEntry],
    ) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn(
                "cm",
                &["merge", "--merge", "--keepdestination" /*revision*/],
            )?
            .wait()?;
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
            Process::spawn(
                "cm",
                &["merge", "--merge", "--keepsource" /*revision*/],
            )?
            .wait()?;
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
        let current_changeset = Process::spawn(
            "cm",
            &[
                "status",
                "--header",
                "--machinereadable",
                "--fieldseparator=\x1f",
            ],
        )?;
        let output = Process::spawn(
            "cm",
            &[
                "find",
                "changeset",
                "--nototal",
                "--format={changesetid}\x1f{date}\x1f{owner}\x1f{branch}\x1f{comment}\x1e",
            ],
        )?;

        let current_changeset = current_changeset.wait()?;
        let current_changeset =
            current_changeset.split('\x1f').nth(1).unwrap_or("");
        let output = output.wait()?;

        let mut entries = Vec::new();
        for record in output.split('\x1e').rev().skip(skip + 1).take(len) {
            let mut splits = record.splitn(5, '\x1f');

            let hash = splits.next().unwrap_or("").trim().into();
            let graph = if hash == current_changeset { "*" } else { " " };
            let graph = graph.into();

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

    fn merge(&self, revision: &str) -> BackendResult<()> {
        let result = Process::spawn("cm", &["merge", "--merge", revision])
            .and_then(Process::wait)
            .map(|_| ());

        // TODO: will this be required?
        if result.is_err() {
            //
        }

        result
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

    fn revision_details(&self, revision: &str) -> BackendResult<RevisionInfo> {
        let output = Process::spawn(
            "cm",
            &[
                "log",
                revision,
                "--csformat={comment}\x1f{items}",
                "--itemformat={shortstatus}\x1f{path}\x1f",
            ],
        )?
        .wait()?;

        let mut splits = output.split('\x1f');
        let message = splits.next().unwrap_or("").into();

        let mut entries = Vec::new();
        loop {
            let status = match splits.next() {
                Some("A") => FileStatus::Added,
                Some("D") => FileStatus::Deleted,
                Some("M") => FileStatus::Renamed,
                Some("C") => FileStatus::Modified,
                _ => break,
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
        _ => panic!("unknown file status '{}'", s),
    }
}
