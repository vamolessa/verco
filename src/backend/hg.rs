use std::path::{Path, PathBuf};

use crate::backend::{
    Backend, BackendResult, BranchEntry, FileStatus, LogEntry, Process, RevisionEntry,
    RevisionInfo, StatusInfo, TagEntry,
};

pub struct Hg;

impl Hg {
    pub fn try_new() -> Option<(PathBuf, Self)> {
        let output = Process::spawn("hg", &["root"]).ok()?.wait().ok()?;
        let root = Path::new(output.trim()).into();
        Some((root, Self))
    }
}

impl Backend for Hg {
    fn status(&self) -> BackendResult<StatusInfo> {
        let header = Process::spawn("hg", &["summary"])?;
        let output = Process::spawn("hg", &["status"])?;

        let header = header.wait()?.lines().next().unwrap_or("").into();
        let output = output.wait()?;

        let mut entries = Vec::new();
        for line in output.lines() {
            let mut splits = line.splitn(2, ' ');
            let status = parse_file_status(splits.next().unwrap_or("").trim());
            let name = splits.next().unwrap_or("").into();

            entries.push(RevisionEntry::new(name, status));
        }

        Ok(StatusInfo { header, entries })
    }

    fn commit(&self, message: &str, entries: &[RevisionEntry]) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("hg", &["commit", "--addremove", "-m", message])?.wait()?;
        } else {
            let mut args = Vec::new();
            args.push("remove");
            for entry in entries {
                if let FileStatus::Missing | FileStatus::Deleted = entry.status {
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

            Process::spawn("hg", &["commit", "-m", message])?.wait()?;
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

    fn diff(&self, revision: Option<&str>, entries: &[RevisionEntry]) -> BackendResult<String> {
        match revision {
            Some(revision) => {
                if entries.is_empty() {
                    Process::spawn("hg", &["diff", "--change", revision])?.wait()
                } else {
                    let mut args = Vec::new();
                    args.push("diff");
                    args.push("--change");
                    args.push(revision);
                    args.push("--");
                    for entry in entries {
                        args.push(&entry.name);
                    }

                    Process::spawn("hg", &args)?.wait()
                }
            }
            None => {
                if entries.is_empty() {
                    Process::spawn("hg", &["diff"])?.wait()
                } else {
                    let mut args = Vec::new();
                    args.push("diff");
                    args.push("--");
                    for entry in entries {
                        args.push(&entry.name);
                    }
                    Process::spawn("hg", &args)?.wait()
                }
            }
        }
    }

    fn resolve_taking_ours(&self, entries: &[RevisionEntry]) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("hg", &["resolve", "--all", "--tool", "internal:local"])?.wait()?;
        } else {
            if !entries
                .iter()
                .any(|e| matches!(e.status, FileStatus::Unmerged))
            {
                return Ok(());
            }

            let mut args = Vec::new();
            args.push("resolve");
            args.push("--tool");
            args.push("internal:local");
            args.push("--");

            for entry in entries {
                if let FileStatus::Unmerged = entry.status {
                    args.push(&entry.name);
                }
            }

            Process::spawn("hg", &args)?.wait()?;
        }

        Ok(())
    }

    fn resolve_taking_theirs(&self, entries: &[RevisionEntry]) -> BackendResult<()> {
        if entries.is_empty() {
            Process::spawn("hg", &["resolve", "--all", "--tool", "internal:other"])?.wait()?;
        } else {
            if !entries
                .iter()
                .any(|e| matches!(e.status, FileStatus::Unmerged))
            {
                return Ok(());
            }

            let mut args = Vec::new();
            args.push("resolve");
            args.push("--tool");
            args.push("internal:other");
            args.push("--");

            for entry in entries {
                if let FileStatus::Unmerged = entry.status {
                    args.push(&entry.name);
                }
            }

            Process::spawn("hg", &args)?.wait()?;
        }

        Ok(())
    }

    fn log(&self, skip: usize, len: usize) -> BackendResult<(usize, Vec<LogEntry>)> {
        let limit = (skip + len).to_string();
        let template = "\x1f{node|short}\x1f{date|shortdate}\x1f{author|person}\x1f{ifeq(phase,'secret','(secret) ','')}{ifeq(phase,'draft','(draft) ','')}{if(topics,'[{topics}] ')}{tags % '{tag} '}{branch}\x1f{desc}";
        let output = Process::spawn(
            "hg",
            &[
                "log",
                "--config",
                "experimental.graphshorten=True",
                "--graph",
                "--template",
                template,
                "--limit",
                &limit,
            ],
        )?
        .wait()?;

        let mut entries = Vec::new();
        for line in output.lines() {
            let mut splits = line.splitn(6, '\x1f');

            let graph = splits.next().unwrap_or("").into();
            let hash = splits.next().unwrap_or("").into();
            let date = splits.next().unwrap_or("").into();
            let author = splits.next().unwrap_or("").into();
            let refs = splits.next().unwrap_or("").into();
            let message = splits.next().unwrap_or("").into();

            entries.push(LogEntry {
                hidden: false,
                graph,
                hash,
                date,
                author,
                refs,
                message,
            });
        }

        Ok((0, entries))
    }

    fn checkout(&self, revision: &str) -> BackendResult<()> {
        Process::spawn("hg", &["update", revision])?.wait()?;
        Ok(())
    }

    fn merge(&self, revision: &str) -> BackendResult<()> {
        Process::spawn("hg", &["merge", revision])?.wait()?;
        Ok(())
    }

    fn fetch(&self) -> BackendResult<()> {
        self.pull()
    }

    fn pull(&self) -> BackendResult<()> {
        Process::spawn("hg", &["pull"])?.wait()?;
        Ok(())
    }

    fn push(&self) -> BackendResult<()> {
        Process::spawn("hg", &["push", "--new-branch"])?.wait()?;
        Ok(())
    }

    fn revision_details(&self, revision: &str) -> BackendResult<RevisionInfo> {
        let message = Process::spawn("hg", &["log", "--rev", revision, "--template", "{desc}"])?;
        let output = Process::spawn("hg", &["status", "--change", revision])?;

        let message = message.wait()?.trim().into();
        let output = output.wait()?;

        let mut entries = Vec::new();
        for line in output.lines() {
            let mut splits = line.splitn(2, ' ');
            let status = parse_file_status(splits.next().unwrap_or("").trim());
            let name = splits.next().unwrap_or("").into();

            entries.push(RevisionEntry::new(name, status));
        }

        Ok(RevisionInfo { message, entries })
    }

    fn branches(&self) -> BackendResult<Vec<BranchEntry>> {
        let entries = Process::spawn("hg", &["branches", "--template", "{branch}\x1f#\\n"])?
            .wait()?
            .lines()
            .map(|l| {
                let mut splits = l.splitn(2, '\x1f');
                let name = splits.next().unwrap_or("").into();
                let checked_out = splits.next().unwrap_or("") == "*";
                BranchEntry { name, checked_out }
            })
            .collect();
        Ok(entries)
    }

    fn new_branch(&self, name: &str) -> BackendResult<()> {
        Process::spawn("hg", &["branch", name])?.wait()?;
        Ok(())
    }

    fn delete_branch(&self, name: &str) -> BackendResult<()> {
        let changeset = Process::spawn("hg", &["identify", "--num"])?.wait()?;
        self.checkout(name)?;
        Process::spawn("hg", &["commit", "-m", "close branch", "--close-branch"])?.wait()?;
        self.checkout(&changeset)?;
        Ok(())
    }

    fn tags(&self) -> BackendResult<Vec<TagEntry>> {
        let entries = Process::spawn("hg", &["tags", "--template", "{tag}\\n"])?
            .wait()?
            .lines()
            .map(|l| TagEntry { name: l.into() })
            .collect();
        Ok(entries)
    }

    fn new_tag(&self, name: &str) -> BackendResult<()> {
        Process::spawn("hg", &["tag", "--force", name])?.wait()?;
        Ok(())
    }

    fn delete_tag(&self, name: &str) -> BackendResult<()> {
        Process::spawn("hg", &["tag", "--remove", name])?.wait()?;
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
        _ => panic!("unknown file status '{}'", s),
    }
}
