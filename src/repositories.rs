use std::env;
use std::path::PathBuf;

use crate::version_control_actions::VersionControlActions;

use crate::git_actions::GitActions;
use crate::hg_actions::HgActions;
use crate::revision_shortcut::RevisionShortcut;

const REPOSITORIES_ENV_VAR_NAME: &str = "VERCO_REPOSITORIES";

pub fn get_version_controls() -> Vec<Box<dyn VersionControlActions>> {
	let mut repositories = match env::var(&REPOSITORIES_ENV_VAR_NAME) {
		Ok(v) => v.split(";").map(|r| String::from(r)).collect(),
		Err(_) => Vec::new(),
	};

	repositories.insert(0, env::current_dir().unwrap().to_str().unwrap().into());
	repositories.dedup();

	let version_controls: Vec<_> = repositories
		.iter()
		.filter_map(|r| {
			let path = PathBuf::from(r);
			let res: Option<Box<dyn VersionControlActions>> = if subdir_exists(&path, ".git") {
				Some(Box::from(GitActions {
					current_dir: r.clone(),
					revision_shortcut: RevisionShortcut::default(),
				}))
			} else if subdir_exists(&path, ".hg") {
				Some(Box::from(HgActions {
					current_dir: r.clone(),
					revision_shortcut: RevisionShortcut::default(),
				}))
			} else {
				None
			};
			res
		})
		.collect();

	version_controls
}

pub fn set_version_controls(version_controls: &Vec<Box<dyn VersionControlActions>>) {
	if version_controls.len() > 0 {
		let directories: Vec<_> = version_controls
			.iter()
			.map(|r| r.repository_directory())
			.collect();
		let repositories = directories.join(";");
		env::set_var(&REPOSITORIES_ENV_VAR_NAME, repositories.clone());
	} else {
		env::remove_var(&REPOSITORIES_ENV_VAR_NAME);
	}
}

fn subdir_exists(basedir: &PathBuf, subdir: &str) -> bool {
	let mut path = basedir.clone();
	path.push(subdir);
	path.exists()
}
