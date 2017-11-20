use termion;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::io::{stdin, stdout, BufRead, Write};

use version_control_actions::VersionControlActions;

pub fn show_tui<'a, T: VersionControlActions>(version_control: &'a T) {
	let _guard = termion::init();

	let stdin = stdin();
	let stdin = stdin.lock();
	let stdout = stdout().into_raw_mode().unwrap();

	Tui::new(stdin, stdout, version_control).show();
}

pub struct Tui<'a, R: BufRead, W: Write, T: VersionControlActions + 'a> {
	stdin: R,
	stdout: W,
	version_control: &'a T,
	readline: Editor<()>,
}

impl<'a, R: BufRead, W: Write, T: VersionControlActions> Tui<'a, R, W, T> {
	pub fn new(stdin: R, stdout: W, version_control: &'a T) -> Self {
		Tui {
			stdin: stdin,
			stdout: stdout,
			version_control: version_control,
			readline: Editor::<()>::new(),
		}
	}

	pub fn show(&mut self) {
		self.show_header();

		loop {
			let key = (&mut self.stdin).keys().next().unwrap().unwrap();

			match key {
				Key::Char('q') => break,
				Key::Ctrl('c') => break,
				Key::Char(key) => self.handle_key(key),
				_ => (),
			}

			self.stdout.flush().unwrap();
		}

		write!(
			self.stdout,
			"{}{}",
			termion::clear::All,
			termion::cursor::Goto(1, 1),
		).unwrap();
	}

	fn show_header(&mut self) {
		write!(
			self.stdout,
			"{}{}Verco - Git/Hg client\n",
			termion::clear::All,
			termion::cursor::Goto(1, 1)
		).unwrap();

		self.stdout.flush().unwrap();
	}

	fn handle_key(&mut self, key: char) {
		match key {
			's' => {
				self.show_action("status");
				self.handle_result(self.version_control.status());
			}
			'l' => {
				self.show_action("log");
				self.handle_result(self.version_control.log());
			}
			'c' => {
				self.show_action("commit");
				if let Some(input) = self.handle_input("commit message (ctrl+c to cancel): ") {
					self.handle_result(self.version_control.commit(&input[..]));
				}
			}
			'r' => {
				self.show_action("revert");
				self.handle_result(self.version_control.revert());
			}
			'u' => {
				self.show_action("update");
				if let Some(input) = self.handle_input("update to (ctrl+c to cancel): ") {
					self.handle_result(self.version_control.update(&input[..]));
				}
			}
			'm' => {
				self.show_action("merge");
				if let Some(input) = self.handle_input("merge with (ctrl+c to cancel): ") {
					self.handle_result(self.version_control.merge(&input[..]));
				}
			}
			'f' => {
				self.show_action("fetch");
				self.handle_result(self.version_control.fetch());
			}
			'p' => {
				self.show_action("pull");
				self.handle_result(self.version_control.pull());
			}
			'P' => {
				self.show_action("push");
				self.handle_result(self.version_control.push());
			}
			't' => {
				self.show_action("tag");
				if let Some(input) = self.handle_input("tag name (ctrl+c to cancel): ") {
					self.handle_result(self.version_control.tag(&input[..]));
				}
			}
			'b' => {
				self.show_action("branch");
				self.handle_result(self.version_control.branches());
			}
			'B' => {
				self.show_action("branch");
				if let Some(input) = self.handle_input("branch name (ctrl+c to cancel): ") {
					self.handle_result(self.version_control.branch(&input[..]));
				}
			}
			_ => (),
		}
	}

	fn show_action(&mut self, action_name: &str) {
		self.show_header();
		write!(self.stdout, "\n{}\n\n", action_name).unwrap();
	}

	fn handle_input(&mut self, prompt: &str) -> Option<String> {
		let readline = self.readline.readline(prompt);
		match readline {
			Ok(line) => Some(line),
			Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
				write!(self.stdout, "\n\ncanceled\n\n").unwrap();
				None
			}
			Err(err) => {
				println!("error {:?}\n\n", err);
				None
			}
		}
	}

	fn handle_result(&mut self, result: Result<String, String>) {
		match result {
			Ok(output) => {
				write!(self.stdout, "{}\n\n", output).unwrap();
				write!(self.stdout, "done\n\n").unwrap();
			}
			Err(error) => {
				write!(self.stdout, "{}\n\n", error).unwrap();
				write!(self.stdout, "error\n\n").unwrap();
			}
		}
	}
}
