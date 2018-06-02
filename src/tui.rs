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
const RESET_BG_COLOR: color::Bg<color::Reset> = color::Bg(color::Reset);

const HEADER_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(0, 0, 0));
const HEADER_BG_COLOR: color::Bg<color::Rgb> = color::Bg(color::Rgb(255, 0, 255));
const ACTION_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 100, 180));
const ENTRY_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 180, 100));

const DONE_COLOR: color::Fg<color::LightGreen> = color::Fg(color::LightGreen);
const CANCEL_COLOR: color::Fg<color::LightYellow> = color::Fg(color::LightYellow);
const ERROR_COLOR: color::Fg<color::Red> = color::Fg(color::Red);

pub fn show_tui<'a, T: VersionControlActions>(repository_name: &str, version_control: &'a T) {
	let _guard = termion::init();

	let stdin = stdin();
	let stdin = stdin.lock();
	let stdout = stdout().into_raw_mode().unwrap();

	Tui::new(stdin, stdout, repository_name, version_control).show();
}

struct Tui<'a, R: BufRead, W: Write, T: VersionControlActions + 'a> {
	stdin: R,
	stdout: W,
	repository_name: &'a str,
	version_control: &'a T,
	readline: Editor<()>,
}

impl<'a, R: BufRead, W: Write, T: VersionControlActions> Tui<'a, R, W, T> {
	fn new(stdin: R, stdout: W, repository_name: &'a str, version_control: &'a T) -> Self {
		Tui {
			stdin: stdin,
			stdout: stdout,
			repository_name: repository_name,
			version_control: version_control,
			readline: Editor::new(),
		}
	}

	fn show(&mut self) {
		self.show_header();
		self.show_help();

		loop {
			let key = (&mut self.stdin).keys().next().unwrap().unwrap();

			match key {
				Key::Char('q') => break,
				Key::Ctrl('c') => break,
				Key::Ctrl(key) => self.handle_key(key, true),
				Key::Char(key) => self.handle_key(key, false),
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

	fn handle_key(&mut self, key: char, is_control_held: bool) {
		if is_control_held {
			match key {
				'b' => {
					self.show_action("close branch");
					if let Some(input) = self.handle_input("branch to close (ctrl+c to cancel): ") {
						self.handle_result(self.version_control.close_branch(&input[..]));
					}
				}
				'R' => {
					self.show_action("merge taking local");
					self.handle_result(self.version_control.take_local());
				}
				_ => (),
			}
		} else {
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
				'd' => {
					self.show_action("revision changes");
					if let Some(input) = self.handle_input("show changes from (ctrl+c to cancel): ")
					{
						self.handle_result(self.version_control.changes(&input[..]));
					}
				}
				'D' => {
					self.show_action("revision diff");
					if let Some(input) = self.handle_input("show diff from (ctrl+c to cancel): ") {
						self.handle_result(self.version_control.diff(&input[..]));
					}
				}
				'c' => {
					self.show_action("commit");
					if let Some(input) = self.handle_input("commit message (ctrl+c to cancel): ") {
						self.handle_result(self.version_control.commit(&input[..]));
					}
				}
				'U' => {
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
				'r' => {
					self.show_action("unresolved conflicts");
					self.handle_result(self.version_control.conflicts());
				}
				'R' => {
					self.show_action("merge taking other");
					self.handle_result(self.version_control.take_other());
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
						self.handle_result(self.version_control.create_tag(&input[..]));
					}
				}
				'b' => {
					self.show_action("branches");
					self.handle_result(self.version_control.list_branches());
				}
				'B' => {
					self.show_action("branch");
					if let Some(input) = self.handle_input("branch name (ctrl+c to cancel): ") {
						self.handle_result(self.version_control.create_branch(&input[..]));
					}
				}
				_ => (),
			}
		}
	}

	fn handle_input(&mut self, prompt: &str) -> Option<String> {
		let readline = self
			.readline
			.readline(&format!("{}{}{}", ENTRY_COLOR, prompt, RESET_COLOR)[..]);

		match readline {
			Ok(line) => Some(line),
			Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
				write!(
					self.stdout,
					"\n\n{}canceled{}\n\n",
					CANCEL_COLOR, RESET_COLOR
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
				write!(self.stdout, "{}done{}\n\n", DONE_COLOR, RESET_COLOR).unwrap();
			}
			Err(error) => {
				write!(self.stdout, "{}\n\n", error).unwrap();
				write!(self.stdout, "{}error{}\n\n", ERROR_COLOR, RESET_COLOR).unwrap();
			}
		}
	}

	fn show_header(&mut self) {
		write!(self.stdout, "{}", termion::clear::All).unwrap();

		if let Ok((w, _)) = termion::terminal_size() {
			write!(
				self.stdout,
				"{}{}{}",
				termion::cursor::Goto(1, 1),
				HEADER_COLOR,
				HEADER_BG_COLOR,
			).unwrap();

			write!(self.stdout, "{}", " ".repeat(w as usize)).unwrap();
		}

		write!(
			self.stdout,
			"{}{}Verco @ {}{}{}\n\n",
			HEADER_COLOR,
			termion::cursor::Goto(1, 1),
			self.repository_name,
			RESET_COLOR,
			RESET_BG_COLOR,
		).unwrap();

		self.stdout.flush().unwrap();
	}

	fn show_action(&mut self, action_name: &str) {
		self.show_header();
		write!(
			self.stdout,
			"{}{}{}\n\n",
			ACTION_COLOR, action_name, RESET_COLOR
		).unwrap();
	}

	fn show_help(&mut self) {
		write!(self.stdout, "Verco 0.5.0\n\n").unwrap();

		match self.version_control.version() {
			Ok(version) => {
				write!(self.stdout, "{}", version).unwrap();
				write!(self.stdout, "\n\n").unwrap();
			}
			Err(error) => {
				write!(self.stdout, "{}{}", ERROR_COLOR, error).unwrap();
				panic!("Could not find version control in system");
			}
		}

		write!(self.stdout, "press a key and peform an action\n\n").unwrap();

		self.show_help_action("h", "help\n");

		self.show_help_action("s", "status");
		self.show_help_action("l", "log\n");

		self.show_help_action("d", "revision changes");
		self.show_help_action("shift+d", "revision diff\n");

		self.show_help_action("c", "commit");
		self.show_help_action("shift+u", "revert");
		self.show_help_action("u", "update/checkout");
		self.show_help_action("m", "merge\n");

		self.show_help_action("r", "unresolved conflicts");
		self.show_help_action("shift+r", "resolve taking other");
		self.show_help_action("ctrl+r", "resolve taking local\n");

		self.show_help_action("f", "fetch");
		self.show_help_action("p", "pull");
		self.show_help_action("shift+p", "push\n");

		self.show_help_action("shift+t", "create tag\n");

		self.show_help_action("b", "list branches");
		self.show_help_action("shift+b", "create branch");
		self.show_help_action("ctrl+b", "close branch\n");
	}

	fn show_help_action(&mut self, shortcut: &str, action: &str) {
		write!(
			self.stdout,
			"\t{}{}{}\t\t{}\n",
			ENTRY_COLOR, shortcut, RESET_COLOR, action
		).unwrap();
	}
}
