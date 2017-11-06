extern crate termion;

use termion::raw::{IntoRawMode, RawTerminal};
use std::io::{stdin, stdout, Stdin, Stdout, Write};

pub struct Tui {
	pub stdin: Stdin,
	pub stdout: RawTerminal<Stdout>,
}

impl Tui {
	pub fn new() -> Tui {
		Tui {
			stdin: stdin(),
			stdout: stdout().into_raw_mode().unwrap(),
		}
	}

	pub fn clear(&mut self) {
		write!(self.stdout, "{}", termion::clear::All).unwrap();
	}
}
