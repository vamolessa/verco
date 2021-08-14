use std::path::PathBuf;

use crate::backend::{Backend, BackendResult, Process};

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

    fn status(&self) -> BackendResult<String> {
        // TODO: parse files
        Process::spawn("git", &["status"])?.wait()
    }
}
