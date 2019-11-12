use std::env;
use std::path::PathBuf;

use crate::version_control_actions::VersionControlActions;

use crate::git_actions::GitActions;
use crate::hg_actions::HgActions;
use crate::revision_shortcut::RevisionShortcut;

pub fn get_current_version_control() -> Option<Box<dyn VersionControlActions>> {
    let current_dir = env::current_dir().unwrap();
    if subdir_exists(&current_dir, ".git") {
        Some(Box::from(GitActions {
            current_dir: current_dir.to_str().unwrap().into(),
            revision_shortcut: RevisionShortcut::default(),
        }))
    } else if subdir_exists(&current_dir, ".hg") {
        Some(Box::from(HgActions {
            current_dir: current_dir.to_str().unwrap().into(),
            revision_shortcut: RevisionShortcut::default(),
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
