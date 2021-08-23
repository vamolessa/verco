use std::{env, path::Path};

use crate::{
    git_actions::GitActions, hg_actions::HgActions,
    version_control_actions::VersionControlActions,
};

pub fn get_current_version_control() -> Option<Box<dyn VersionControlActions>> {
    let mut args = env::args();
    if let Some(dir) = args.nth(1) {
        let dir = Path::new(&dir);
        if dir.canonicalize().is_err() {
            eprintln!("{:?} is not a valid directory", dir);
            return None;
        }

        env::set_current_dir(dir).expect("could not set current directory");
    }

    let current_dir =
        env::current_dir().expect("could not get current directory");
    let current_dir = match current_dir.to_str() {
        Some(current_dir) => current_dir,
        None => {
            eprintln!("{:?} is not valid utf8", current_dir);
            return None;
        }
    };

    // first try Git because it's the most common and also responds the fastest
    let mut git_actions = Box::from(GitActions {
        current_dir: current_dir.into(),
    });
    if git_actions.set_root().is_ok() {
        return Some(git_actions);
    }

    // otherwise try Mercurial
    let mut hg_actions = Box::from(HgActions {
        current_dir: current_dir.into(),
    });
    if hg_actions.set_root().is_ok() {
        return Some(hg_actions);
    }

    eprintln!("no repository found");
    None
}
