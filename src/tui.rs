extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use std::io::{stdin, stdout, Write};

use version_control::VersionControl;
use actions::Action;

pub fn show(version_control: &VersionControl) {
	let stdin = stdin();
	let mut stdout = stdout().into_raw_mode().unwrap();

	write!(
		stdout,
		"{}{}q to exit. Type stuff, use alt, and so on.",
		termion::clear::All,
		termion::cursor::Goto(1, 1)
	).unwrap();

	stdout.flush().unwrap();

	for c in stdin.keys() {
		write!(
			stdout,
			"{}{}",
			termion::clear::All,
			termion::cursor::Goto(1, 1)
		).unwrap();

		match c.unwrap() {
			Key::Ctrl('c') => break,

			Key::Char(key) => handle_key(version_control, key, &mut stdout),
			Key::Ctrl(c) => println!("ctrl+{}", c),

			_ => (),
		}

		stdout.flush().unwrap();
	}
}

fn handle_key<T: Write>(version_control: &VersionControl, key: char, stdout: &mut T) {
	match version_control
		.actions
		.iter()
		.find(|a| a.key.starts_with(key))
	{
		Some(action) => handle_action(version_control, action, stdout),
		None => (),
	};
}

fn handle_action<T: Write>(version_control: &VersionControl, action: &Action, stdout: &mut T) {
	write!(
		stdout,
		"{}executing {}\n\n",
		termion::clear::All,
		action.name
	).unwrap();

	match version_control.run_action(&action.name[..]) {
		Ok(output) => write!(stdout, "{}\n\ndone\n\n", output),
		Err(error) => write!(stdout, "{}\n\nerror\n\n", error),
	}.unwrap();
}
