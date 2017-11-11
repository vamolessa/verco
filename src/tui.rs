extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use std::io::{stdin, stdout, Write};

use version_control::VersionControl;
use actions::Action;

pub struct Tui<'a> {
	version_control: &'a VersionControl<'a>,
}

impl<'a> Tui<'a> {
	pub fn new(version_control: &'a VersionControl) -> Tui<'a> {
		Tui {
			version_control: version_control,
		}
	}

	pub fn show(&self) {
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
			match c.unwrap() {
				Key::Char('q') => break,
				Key::Ctrl('c') => break,
				Key::Char(key) => self.handle_key(key, &mut stdout),
				_ => (),
			}

			stdout.flush().unwrap();
		}
	}

	fn handle_key<T: Write>(&self, key: char, stdout: &mut T) {
		match self.version_control
			.actions
			.iter()
			.find(|a| a.key.starts_with(key))
		{
			Some(action) => self.handle_action(action, stdout),
			None => (),
		};
	}

	fn handle_action<T: Write>(&self, action: &Action, stdout: &mut T) {
		write!(
			stdout,
			"{}{}action {}\n\n",
			termion::clear::All,
			termion::cursor::Goto(1, 1),
			action.name
		).unwrap();

		match self.version_control.run_action(
			&action.name[..],
			|| String::from(""),
			|output| match output {
				Ok(output) => {
					write!(stdout, "{}\n\n", output).unwrap();
					write!(stdout, "done\n\n").unwrap();
				}
				Err(error) => {
					write!(stdout, "{}\n\n", error).unwrap();
					write!(stdout, "error\n\n").unwrap();
				}
			},
		) {
			Ok(_) => {
				write!(stdout, "done\n\n").unwrap();
			}
			Err(_) => {
				write!(stdout, "error\n\n").unwrap();
			}
		};
	}
}
