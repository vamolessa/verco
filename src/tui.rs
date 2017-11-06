extern crate termion;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use std::io::{stdin, stdout, Error, Stdin, Stdout, Write};

use version_control::{Action, VersionControl};

pub struct Tui {
	reader: Stdin,
	writer: RawTerminal<Stdout>,
}

impl Tui {
	pub fn init() -> Tui {
		Tui {
			reader: stdin(),
			writer: stdout().into_raw_mode().unwrap(),
		}
	}

	pub fn show(&mut self, version_control: &VersionControl) {
		write!(
			self.writer,
			"{}{}q to exit. Type stuff, use alt, and so on.{}",
			termion::clear::All,
			termion::cursor::Goto(1, 1),
			termion::cursor::Hide
		).unwrap();

		self.flush();

		let keys: Vec<Result<Key, Error>> = (&mut self.reader).keys().collect();

		for c in keys {
			write!(
				self.writer,
				"{}{}",
				termion::cursor::Goto(1, 1),
				termion::clear::CurrentLine
			).unwrap();

			match c.unwrap() {
				Key::Ctrl('c') => break,

				Key::Ctrl('s') => self.show_action(version_control, Action::Status),

				Key::Char(c) => println!("{}", c),
				Key::Ctrl(c) => println!("ctrl+{}", c),

				_ => (),
			}

			self.flush();
		}

		write!(self.writer, "{}", termion::cursor::Show).unwrap();
	}

	fn show_action(&mut self, version_control: &VersionControl, action: Action) {
		match version_control.on_action(action) {
			Ok(result) => write!(self.writer, "{}", result),
			Err(error) => write!(self.writer, "{}", error),
		}.unwrap();
	}

	fn clear(&mut self) {
		write!(self.writer, "{}", termion::clear::All).unwrap();
	}

	fn flush(&mut self) {
		self.writer.flush().unwrap();
	}
}
