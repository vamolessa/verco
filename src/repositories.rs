use std::env;
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::PathBuf;

use crate::version_control_actions::VersionControlActions;

use crate::git_actions::GitActions;
use crate::hg_actions::HgActions;
use crate::revision_shortcut::RevisionShortcut;

pub fn get_version_controls() -> io::Result<Vec<Box<dyn VersionControlActions>>> {
	let path = get_repositories_path();
	let mut contents = String::new();
	if path.exists() {
		let mut file = File::open(path)?;
		file.read_to_string(&mut contents)?;
	}

	let mut repositories = if contents.is_empty() {
		Vec::new()
	} else {
		contents.split(";").map(|r| String::from(r)).collect()
	};

	{
		let current_dir = env::current_dir().unwrap();
		let current_repository = current_dir.to_str().unwrap();
		repositories.push(current_repository.into());
		repositories.sort();
		repositories.dedup();

		if let Some(current_repository_index) =
			repositories.iter().position(|r| r == current_repository)
		{
			repositories.swap(0, current_repository_index);
		}
	}

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

	Ok(version_controls)
}

pub fn set_version_controls(
	version_controls: &Vec<Box<dyn VersionControlActions>>,
) -> io::Result<()> {
	let directories: Vec<_> = version_controls
		.iter()
		.map(|r| r.repository_directory())
		.collect();
	let contents = directories.join(";");

	let mut file = File::create(get_repositories_path())?;
	file.write_all(contents.as_bytes())?;
	Ok(())
}

fn subdir_exists(basedir: &PathBuf, subdir: &str) -> bool {
	let mut path = basedir.clone();
	path.push(subdir);
	path.exists()
}

fn get_repositories_path() -> PathBuf {
	let exe_path = env::current_exe().unwrap();
	let directory = exe_path.parent().unwrap();
	let mut path = PathBuf::from(directory);
	path.push("repositories");
	path
}
