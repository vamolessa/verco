use std::env;
use std::path::PathBuf;

mod git_actions;
mod hg_actions;
mod revision_shortcut;
mod select;
mod tui;
mod version_control_actions;

use git_actions::GitActions;
use hg_actions::HgActions;
use revision_shortcut::RevisionShortcut;
use version_control_actions::VersionControlActions;

const ENV_VAR: &str = "VERCO_REPOSITORIES";

fn main() {
	ctrlc::set_handler(move || {}).unwrap();

	let mut repositories: Vec<_> = env::var(&ENV_VAR)
		.unwrap_or_default()
		.split(";")
		.map(|r| String::from(r))
		.collect();

	let current_dir_path = env::current_dir().unwrap();
	let current_dir = current_dir_path.to_str().unwrap();
	repositories.push(current_dir.into());

	let version_controls: Vec<_> = repositories
		.iter()
		.filter_map(|r| {
			let path = PathBuf::from(r);
			let res: Option<Box<dyn VersionControlActions>> = if subdir_exists(&path, ".git") {
				Some(Box::from(GitActions {
					current_dir: current_dir.into(),
					revision_shortcut: RevisionShortcut::default(),
				}))
			} else if subdir_exists(&path, ".hg") {
				Some(Box::from(HgActions {
					current_dir: current_dir.into(),
					revision_shortcut: RevisionShortcut::default(),
				}))
			} else {
				None
			};
			res
		})
		.collect();

	if version_controls.len() == 0 {
		println!("no repository found");
	} else {
		tui::show_tui(version_controls);
	}
}

fn subdir_exists(basedir: &PathBuf, subdir: &str) -> bool {
	let mut path = basedir.clone();
	path.push(subdir);
	path.exists()
}
