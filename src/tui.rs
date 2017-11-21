use termion;
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::io::{stdin, stdout, BufRead, Write};

use version_control_actions::VersionControlActions;

const RESET_COLOR: color::Fg<color::Reset> = color::Fg(color::Reset);

const HEADER_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 0, 255));
const ACTION_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 100, 180));
const ENTRY_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 180, 100));

const DONE_COLOR: color::Fg<color::LightGreen> = color::Fg(color::LightGreen);
const CANCEL_COLOR: color::Fg<color::LightYellow> = color::Fg(color::LightYellow);
const ERROR_COLOR: color::Fg<color::Red> = color::Fg(color::Red);

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
		self.show_help();

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

	fn handle_key(&mut self, key: char) {
		match key {
			'h' => {
				self.show_action("help");
				self.show_help();
			}
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
			'R' => {
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
			'T' => {
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

	fn handle_input(&mut self, prompt: &str) -> Option<String> {
		let readline = self.readline
			.readline(&format!("{}{}{}", ENTRY_COLOR, prompt, RESET_COLOR)[..]);

		match readline {
			Ok(line) => Some(line),
			Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
				write!(
					self.stdout,
					"\n\n{}canceled{}\n\n",
					CANCEL_COLOR,
					RESET_COLOR
				).unwrap();
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
				write!(
					self.stdout,
					"{}done{}\n\n",
					DONE_COLOR,
					RESET_COLOR
				).unwrap();
			}
			Err(error) => {
				write!(self.stdout, "{}\n\n", error).unwrap();
				write!(
					self.stdout,
					"{}error{}\n\n",
					ERROR_COLOR,
					RESET_COLOR
				).unwrap();
			}
		}
	}

	fn show_header(&mut self) {
		write!(
			self.stdout,
			"{}{}{}Verco - Git/Hg client{}\n\n",
			termion::clear::All,
			termion::cursor::Goto(1, 1),
			HEADER_COLOR,
			RESET_COLOR
		).unwrap();

		self.stdout.flush().unwrap();
	}

	fn show_action(&mut self, action_name: &str) {
		self.show_header();
		write!(
			self.stdout,
			"{}{}{}\n\n",
			ACTION_COLOR,
			action_name,
			RESET_COLOR
		).unwrap();
	}

	fn show_help(&mut self) {
		write!(self.stdout, "press a key and peform an action\n\n").unwrap();

		self.show_help_action("h", "help\n");

		self.show_help_action("s", "status");
		self.show_help_action("l", "log\n");

		self.show_help_action("c", "commit");
		self.show_help_action("shift+r", "revert");
		self.show_help_action("u", "update");
		self.show_help_action("m", "merge\n");

		self.show_help_action("f", "fetch");
		self.show_help_action("p", "pull");
		self.show_help_action("shift+p", "push\n");

		self.show_help_action("shift+t", "tag");
		self.show_help_action("b", "branches");
		self.show_help_action("shift+b", "branch\n");
	}

	fn show_help_action(&mut self, shortcut: &str, action: &str) {
		write!(
			self.stdout,
			"\t{}{}{}\t\t{}\n",
			ENTRY_COLOR,
			shortcut,
			RESET_COLOR,
			action
		).unwrap();
	}
}
