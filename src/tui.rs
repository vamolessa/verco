use termion;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use liner::Context;

use std::io::{stdin, stdout, BufRead, Write};

use version_control::{VersionControl, VersionControlIO};
use actions::Action;

pub fn show_tui<'a>(version_control: &'a VersionControl) {
	let _guard = termion::init();

	let stdin = stdin();
	let stdin = stdin.lock();
	let stdout = stdout().into_raw_mode().unwrap();

	Tui::new(stdin, stdout, version_control).show();
}

pub struct Tui<'a, R: BufRead, W: Write> {
	stdin: R,
	stdout: W,
	version_control: &'a VersionControl<'a>,
	liner_context: Context,
}

impl<'a, R: BufRead, W: Write> VersionControlIO for Tui<'a, R, W> {
	fn on_in(&mut self) -> Option<String> {
		let line = self.liner_context.read_line("", &mut |_| {}).unwrap();

		if line.is_empty() {
			return None;
		}

		//self.liner_context.history.push(line.into()).unwrap();
		Some(line)

	}

	fn on_out(&mut self, result: Result<Option<&str>, &str>) {
		match result {
			Ok(output) => match output {
				Some(output) => {
					write!(self.stdout, "\n\n{}\n\n", output).unwrap();
					write!(self.stdout, "done\n\n").unwrap();
				}
				None => {
					write!(self.stdout, "\n\ncancelled\n\n").unwrap();
				}
			},
			Err(error) => {
				write!(self.stdout, "{}\n\n", error).unwrap();
				write!(self.stdout, "error\n\n").unwrap();
			}
		}
	}
}

impl<'a, R: BufRead, W: Write> Tui<'a, R, W> {
	pub fn new(stdin: R, stdout: W, version_control: &'a VersionControl) -> Tui<'a, R, W> {
		Tui {
			stdin: stdin,
			stdout: stdout,
			version_control: version_control,
			liner_context: Context::new(),
		}
	}

	pub fn show(&mut self) {
		write!(
			self.stdout,
			"{}{}q to exit. Type stuff, use alt, and so on.",
			termion::clear::All,
			termion::cursor::Goto(1, 1)
		).unwrap();

		self.stdout.flush().unwrap();

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
	}

	fn handle_key(&mut self, key: char) {
		match self.version_control
			.actions
			.iter()
			.find(|a| a.key.starts_with(key))
		{
			Some(action) => self.handle_action(action),
			None => (),
		};
	}

	fn handle_action(&mut self, action: &Action) {
		write!(
			self.stdout,
			"{}{}action {}\n\n",
			termion::clear::All,
			termion::cursor::Goto(1, 1),
			action.name
		).unwrap();

		match self.version_control.run_action(&action.name[..], self) {
			Ok(_) => (),
			Err(error) => {
				write!(self.stdout, "{}\n\nerror\n\n", error).unwrap();
			}
		};
	}
}
