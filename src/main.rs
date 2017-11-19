extern crate termion;
extern crate rustyline;

use std::env;

mod tui;
mod version_control_actions;
mod git_actions;

use git_actions::GitActions;

fn main() {
	let current_dir_path = env::current_dir().unwrap();
	let current_dir = current_dir_path.to_str().unwrap();

	let git_actions = GitActions {
		current_dir: &current_dir,
	};

	tui::show_tui(&git_actions);
}
