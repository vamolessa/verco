use std::env;
use std::path::PathBuf;

mod git_actions;
mod hg_actions;
mod revision_shortcut;
mod select;
mod tui;
mod version_control_actions;
//mod keybindings;

use git_actions::GitActions;
use hg_actions::HgActions;
use revision_shortcut::RevisionShortcut;

fn main() {
	let current_dir_path = env::current_dir().unwrap();
	let current_dir = current_dir_path.to_str().unwrap();

	let revision_shortcut = RevisionShortcut::default();

	if subdir_exists(&current_dir_path, ".git") {
		let mut actions = GitActions {
			current_dir: &current_dir,
			revision_shortcut: revision_shortcut,
		};
		tui::show_tui(&current_dir, &mut actions);
	} else if subdir_exists(&current_dir_path, ".hg") {
		let mut actions = HgActions {
			current_dir: &current_dir,
			revision_shortcut: revision_shortcut,
		};
		tui::show_tui(&current_dir, &mut actions);
	} else {
		println!("no repository found");
	}
}

fn subdir_exists(basedir: &PathBuf, subdir: &str) -> bool {
	let mut path = basedir.clone();
	path.push(subdir);
	path.exists()
}
