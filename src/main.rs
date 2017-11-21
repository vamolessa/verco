extern crate rustyline;
extern crate termion;

use std::env;
use std::path::PathBuf;

mod tui;
mod version_control_actions;
mod git_actions;
mod hg_actions;

use git_actions::GitActions;
use hg_actions::HgActions;

fn main() {
	let current_dir_path = env::current_dir().unwrap();
	let current_dir = current_dir_path.to_str().unwrap();

	if subdir_exists(&current_dir_path, ".git") {
		let actions = GitActions {
			current_dir: &current_dir,
		};
		tui::show_tui(&actions);
	} else if subdir_exists(&current_dir_path, ".hg") {
		let actions = HgActions {
			current_dir: &current_dir,
		};
		tui::show_tui(&actions);
	} else {
		println!("no repository found");
	}
}

fn subdir_exists(basedir: &PathBuf, subdir: &str) -> bool {
	let mut path = basedir.clone();
	path.push(subdir);
	path.exists()
}
