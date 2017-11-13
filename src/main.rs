extern crate liner;
extern crate serde;
extern crate serde_json;
extern crate termion;

#[macro_use]
extern crate serde_derive;

use std::env;

mod tui;
mod version_control;
mod actions;

use version_control::VersionControl;
use actions::Actions;

fn main() {
	{
		let mut con = liner::Context::new();

		loop {
			let res = con.read_line("[prompt]$ ", &mut |_| {}).unwrap();

			if res.is_empty() {
				break;
			}

			con.history.push(res.into()).unwrap();
		}
	}

	let default_actions_json = include_str!("default_actions.json");
	let actions = Actions::from_json(default_actions_json).unwrap();

	let current_dir_path = env::current_dir().unwrap();
	let current_dir = current_dir_path.to_str().unwrap();

	match VersionControl::find_current(current_dir, &actions) {
		Ok(version_control) => tui::show_tui(&version_control),
		Err(_) => println!("Not on a valid repository"),
	}
}
