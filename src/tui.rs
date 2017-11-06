extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use std::io::{stdin, stdout, Write};

use version_control::{Action, VersionControl};

pub fn show(version_control: &VersionControl) {
	let stdin = stdin();
	let mut stdout = stdout().into_raw_mode().unwrap();

	write!(
		stdout,
		"{}{}q to exit. Type stuff, use alt, and so on.{}",
		termion::clear::All,
		termion::cursor::Goto(1, 1),
		termion::cursor::Hide
	).unwrap();

	flush(&mut stdout);

	for c in stdin.keys() {
		write!(
			stdout,
			"{}{}",
			termion::cursor::Goto(1, 1),
			termion::clear::CurrentLine
		).unwrap();

		match c.unwrap() {
			Key::Ctrl('c') => break,

			Key::Ctrl('s') => show_action(version_control, Action::Status, &mut stdout),

			Key::Char(c) => println!("{}", c),
			Key::Ctrl(c) => println!("ctrl+{}", c),

			_ => (),
		}

		flush(&mut stdout);
	}

	write!(stdout, "{}", termion::cursor::Show).unwrap();
}

fn show_action<T: Write>(version_control: &VersionControl, action: Action, stdout: &mut T) {
	match version_control.on_action(action) {
		Ok(result) => write!(stdout, "{}", result),
		Err(error) => write!(stdout, "{}", error),
	}.unwrap();
}

fn flush<T: Write>(stdout: &mut T) {
	stdout.flush().unwrap();
}
