extern crate liner;
extern crate termion;

use std::env;

mod tui;
mod git_actions;

use git_actions::GitActions;

fn main() {
	let current_dir_path = env::current_dir().unwrap();
	let current_dir = current_dir_path.to_str().unwrap();

	let git_actions = GitActions {
		current_dir: &current_dir,
	};

	match git_actions.status() {
		Ok(output) => println!("status:\n{}", output),
		Err(error) => println!("status deu ruim:\n{}", error),
	}

/*
	match VersionControl::find_current(current_dir, &actions) {
		Ok(version_control) => tui::show_tui(&version_control),
		Err(_) => println!("Not on a valid repository"),
	}
	*/
}
