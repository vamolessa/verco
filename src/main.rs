extern crate termion;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::env;

mod tui;
mod version_control;
mod actions;

use version_control::VersionControl;
use actions::Actions;

fn main() {
	let default_actions_json = include_str!("default_actions.json");
	let actions = Actions::from_json(default_actions_json).unwrap();

	let current_dir_path = env::current_dir().unwrap();
	let current_dir = current_dir_path.to_str().unwrap();

	match VersionControl::find_current(current_dir, &actions) {
		Ok(version_control) => tui::show_tui(&version_control),
		Err(_) => println!("Not on a valid repository"),
	}
}
