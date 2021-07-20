use std::path::PathBuf;

use crate::backend::{get_command_output, Backend, Context};

pub struct Git {
    //
}

impl Git {
    pub fn try_new() -> Option<(PathBuf, Self)> {
        let dir = get_command_output("git", &["rev-parse", "--show-toplevel"])?;
        let dir = dir.lines().next()?;
        let mut root = PathBuf::new();
        root.push(dir);
        Some((root, Self {}))
    }
}

impl Backend for Git {
    fn status(&mut self, ctx: &mut Context) {
        //
    }
}

