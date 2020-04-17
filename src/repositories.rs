use std::{env, path::{Path, PathBuf}};

use crate::git_actions::GitActions;
use crate::hg_actions::HgActions;
use crate::version_control_actions::VersionControlActions;

pub fn get_current_version_control() -> Option<Box<dyn VersionControlActions>> {
    let mut args = env::args();
    if let Some(dir) = args.nth(1) {
        let dir = Path::new(&dir);
        dir.canonicalize().expect("could not canonicalize path");
        env::set_current_dir(dir).expect("could not set current directory");
    }

    let current_dir = env::current_dir().expect("could not get current directory");

    if subdir_exists(&current_dir, ".git") {
        Some(Box::from(GitActions {
            current_dir: current_dir.to_str().expect("current directory is not valid utf8").into(),
            revision_shortcut: Default::default(),
        }))
    } else if subdir_exists(&current_dir, ".hg") {
        Some(Box::from(HgActions {
            current_dir: current_dir.to_str().expect("current directory is not valid utf8").into(),
            revision_shortcut: Default::default(),
        }))
    } else {
        None
    }
}

fn subdir_exists(basedir: &PathBuf, subdir: &str) -> bool {
    let mut path = basedir.clone();
    path.push(subdir);
    path.exists()
}
