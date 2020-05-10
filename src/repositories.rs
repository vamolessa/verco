use std::{env, path::Path};

use crate::{
    git_actions::GitActions, hg_actions::HgActions,
    version_control_actions::VersionControlActions,
};

pub fn get_current_version_control() -> Option<Box<dyn VersionControlActions>> {
    let mut args = env::args();
    if let Some(dir) = args.nth(1) {
        let dir = Path::new(&dir);
        dir.canonicalize().expect("could not canonicalize path");
        env::set_current_dir(dir).expect("could not set current directory");
    }

    let current_dir =
        env::current_dir().expect("could not get current directory");

    // First try Git because it's the most common and also responds the fastest
    let mut git_actions = Box::from(GitActions {
        current_dir: current_dir
            .to_str()
            .expect("current directory is not valid utf8")
            .into(),
        revision_shortcut: Default::default(),
    });

    if git_actions.set_root().is_ok() {
        return Some(git_actions);
    }

    // Otherwise try Mercurial
    let mut hg_actions = Box::from(HgActions {
        current_dir: current_dir
            .to_str()
            .expect("current directory is not valid utf8")
            .into(),
        revision_shortcut: Default::default(),
    });

    if hg_actions.set_root().is_ok() {
        return Some(hg_actions);
    }

    None
}
