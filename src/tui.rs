use crossterm::*;

use std::process::Command;

use crate::select::{select, Entry};
use crate::version_control_actions::VersionControlActions;
const RESET_COLOR: Attribute = Attribute::Reset;
const HEADER_COLOR: Colored = Colored::Fg(Color::Black);
const HEADER_BG_COLOR: Colored = Colored::Bg(Color::Magenta);
const ACTION_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 255,
	g: 100,
	b: 180,
});
const ENTRY_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 255,
	g: 180,
	b: 100,
});

const DONE_COLOR: Colored = Colored::Fg(Color::Green);
const CANCEL_COLOR: Colored = Colored::Fg(Color::Yellow);
const ERROR_COLOR: Colored = Colored::Fg(Color::Red);

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn show_tui<'a, T: VersionControlActions>(repository_name: &str, version_control: &'a mut T) {
	Tui::new(repository_name, version_control).show();
}

struct Tui<'a, T: VersionControlActions + 'a> {
	repository_name: &'a str,
	version_control: &'a mut T,

	_crossterm: Crossterm,
	terminal: Terminal,
	input: TerminalInput,
	cursor: TerminalCursor,
}

impl<'a, T: VersionControlActions> Tui<'a, T> {
	fn new(repository_name: &'a str, version_control: &'a mut T) -> Self {
		let crossterm = Crossterm::new();
		let terminal = crossterm.terminal();
		let input = crossterm.input();
		let cursor = crossterm.cursor();

		Tui {
			repository_name: repository_name,
			version_control: version_control,

			_crossterm: crossterm,
			terminal,
			input,
			cursor,
		}
	}

	fn show(&mut self) {
		self.show_header();
		self.show_help();

		let mut ignore_next = false;
		loop {
			match self.input.read_char() {
				Ok(key) => {
					self.terminal.clear(ClearType::CurrentLine).unwrap();
					self.cursor.move_left(1);

					if ignore_next {
						ignore_next = false;
						continue;
					}

					if !self.handle_key(key) {
						break;
					}
				}
				Err(_error) => {
					ignore_next = true;
				}
			}
		}

		self.cursor.show().unwrap();
	}

	fn handle_key(&mut self, key: char) -> bool {
		match key {
			// ctrl+c
			'q' | '\x03' => {
				return false;
			}
			'h' => {
				self.show_action("help");
				self.show_help();
			}
			'e' => {
				self.show_action("explorer");
				self.open_explorer();
			}
			's' => {
				self.show_action("status");
				let result = self.version_control.status();
				self.handle_result(result);
			}
			'l' => {
				self.show_action("log");
				let result = self.version_control.log();
				self.handle_result(result);
			}
			'd' => {
				self.show_action("revision changes");
				if let Some(input) = self.handle_input("show changes from (ctrl+c to cancel): ") {
					let result = self.version_control.changes(&input[..]);
					self.handle_result(result);
				}
			}
			'D' => {
				self.show_action("revision diff");
				if let Some(input) = self.handle_input("show diff from (ctrl+c to cancel): ") {
					let result = self.version_control.diff(&input[..]);
					self.handle_result(result);
				}
			}
			'c' => {
				self.show_action("commit all");

				if let Some(input) = self.handle_input("commit message (ctrl+c to cancel): ") {
					let result = self.version_control.commit_all(&input[..]);
					self.handle_result(result);
				}
			}
			'C' => {
				self.show_action("commit selected");
				match self.version_control.get_files_to_commit() {
					Ok(mut entries) => {
						if self.show_select_ui(&mut entries) {
							print!("\n\n");

							if let Some(input) =
								self.handle_input("commit message (ctrl+c to cancel): ")
							{
								let result =
									self.version_control.commit_selected(&input[..], &entries);
								self.handle_result(result);
							}
						}
					}
					Err(error) => self.handle_result(Err(error)),
				}
			}
			'u' => {
				self.show_action("update");
				if let Some(input) = self.handle_input("update to (ctrl+c to cancel): ") {
					let result = self.version_control.update(&input[..]);
					self.handle_result(result);
				}
			}
			// ctrl+x
			'\x24' => {
				self.show_action("revert all");
				let result = self.version_control.revert_all();
				self.handle_result(result);
			}
			'X' => {
				self.show_action("revert selected");
				match self.version_control.get_files_to_commit() {
					Ok(mut entries) => {
						if self.show_select_ui(&mut entries) {
							print!("\n\n");
							let result = self.version_control.revert_selected(&entries);
							self.handle_result(result);
						}
					}
					Err(error) => self.handle_result(Err(error)),
				}
			}
			'm' => {
				self.show_action("merge");
				if let Some(input) = self.handle_input("merge with (ctrl+c to cancel): ") {
					let result = self.version_control.merge(&input[..]);
					self.handle_result(result);
				}
			}
			'r' => {
				self.show_action("unresolved conflicts");
				let result = self.version_control.conflicts();
				self.handle_result(result);
			}
			'R' => {
				self.show_action("merge taking other");
				let result = self.version_control.take_other();
				self.handle_result(result);
			}
			// ctrl+r
			'\x12' => {
				self.show_action("merge taking local");
				let result = self.version_control.take_local();
				self.handle_result(result);
			}
			'f' => {
				self.show_action("fetch");
				let result = self.version_control.fetch();
				self.handle_result(result);
			}
			'p' => {
				self.show_action("pull");
				let result = self.version_control.pull();
				self.handle_result(result);
			}
			'P' => {
				self.show_action("push");
				let result = self.version_control.push();
				self.handle_result(result);
			}
			'T' => {
				self.show_action("create tag");
				if let Some(input) = self.handle_input("tag name (ctrl+c to cancel): ") {
					let result = self.version_control.create_tag(&input[..]);
					self.handle_result(result);
				}
			}
			'b' => {
				self.show_action("list branches");
				let result = self.version_control.list_branches();
				self.handle_result(result);
			}
			'B' => {
				self.show_action("create branch");
				if let Some(input) = self.handle_input("branch name (ctrl+c to cancel): ") {
					let result = self.version_control.create_branch(&input[..]);
					self.handle_result(result);
				}
			}
			// ctrl+b
			'\x02' => {
				self.show_action("close branch");
				if let Some(input) = self.handle_input("branch to close (ctrl+c to cancel): ") {
					let result = self.version_control.close_branch(&input[..]);
					self.handle_result(result);
				}
			}
			_ => (),
		}

		true
	}

	fn handle_input(&mut self, prompt: &str) -> Option<String> {
		print!("{}{}{}\n", ENTRY_COLOR, prompt, RESET_COLOR);
		self.cursor.show().unwrap();
		let res = match self.input.read_line() {
			Ok(line) => {
				if line.len() > 0 {
					Some(line)
				} else {
					None
				}
			}
			Err(_error) => None,
		};

		if res.is_none() {
			print!("\n\n{}canceled{}\n\n", CANCEL_COLOR, RESET_COLOR);
		}

		self.cursor.hide().unwrap();
		res
	}

	fn handle_result(&mut self, result: std::result::Result<String, String>) {
		match result {
			Ok(output) => {
				print!("{}\n\n", output);
				print!("{}done{}\n\n", DONE_COLOR, RESET_COLOR);
			}
			Err(error) => {
				print!("{}\n\n", error);
				print!("{}error{}\n\n", ERROR_COLOR, RESET_COLOR);
			}
		}
	}

	fn show_header(&mut self) {
		self.terminal.clear(ClearType::All).unwrap();

		let (w, _) = self.terminal.terminal_size();
		self.cursor.goto(0, 0).unwrap();
		print!("{}{}", HEADER_COLOR, HEADER_BG_COLOR,);
		print!("{}", " ".repeat(w as usize));

		self.cursor.goto(0, 0).unwrap();
		print!(
			"{}Verco @ {}{}{}\n\n",
			HEADER_COLOR, self.repository_name, RESET_COLOR, RESET_COLOR,
		);
	}

	fn show_action(&mut self, action_name: &str) {
		self.show_header();
		print!("{}{}{}\n\n", ACTION_COLOR, action_name, RESET_COLOR);
	}

	fn show_help(&mut self) {
		print!("Verco {}\n\n", VERSION);

		match self.version_control.version() {
			Ok(version) => {
				print!("{}", version);
				print!("\n\n");
			}
			Err(error) => {
				print!("{}{}", ERROR_COLOR, error);
				panic!("Could not find version control in system");
			}
		}

		print!("press a key and peform an action\n\n");

		self.show_help_action("h", "help\n");

		self.show_help_action("e", "explorer\n");

		self.show_help_action("s", "status");
		self.show_help_action("l", "log\n");

		self.show_help_action("d", "revision changes");
		self.show_help_action("shift+d", "revision diff\n");

		self.show_help_action("c", "commit all");
		self.show_help_action("shift+c", "commit selected");
		self.show_help_action("ctrl+x", "revert all");
		self.show_help_action("shift+x", "revert selected");
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
		print!(
			"\t{}{}{}\t\t{}\n",
			ENTRY_COLOR, shortcut, RESET_COLOR, action
		);
	}

	fn open_explorer(&mut self) {
		let mut command = Command::new("explorer");
		command.arg(self.repository_name);
		command.spawn().expect("failed to open explorer");

		print!("{}done{}\n\n", DONE_COLOR, RESET_COLOR);
	}

	pub fn show_select_ui(&mut self, entries: &mut Vec<Entry>) -> bool {
		if select(
			&mut self.terminal,
			&mut self.cursor,
			&mut self.input,
			entries,
		) {
			true
		} else {
			print!("\n\n{}canceled{}\n\n", CANCEL_COLOR, RESET_COLOR);
			false
		}
	}
}
