use termion;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

use std::io::{stdin, stdout, Read, Write};

use version_control::VersionControl;
use actions::Action;

pub fn show_tui<'a>(version_control: &'a VersionControl) {
	let _guard = termion::init();

	let stdin = stdin();
	let stdin = stdin.lock();
	let stdout = stdout().into_raw_mode().unwrap();

	Tui::new(stdin, stdout, version_control).show();
}

pub struct Tui<'a, R: Read, W: Write> {
	stdin: R,
	stdout: W,
	version_control: &'a VersionControl<'a>,
}

impl<'a, R: Read, W: Write> Tui<'a, R, W> {
	pub fn new(stdin: R, stdout: W, version_control: &'a VersionControl) -> Tui<'a, R, W> {
		Tui {
			stdin: stdin,
			stdout: stdout,
			version_control: version_control,
		}
	}

	pub fn show(&mut self) {
		write!(
			self.stdout,
			"{}{}q to exit. Type stuff, use alt, and so on.",
			termion::clear::All,
			termion::cursor::Goto(1, 1)
		).unwrap();

		let pass = self.stdin.read_line();
		if let Ok(Some(pass)) = pass {
			self.stdout.write_all(pass.as_bytes()).unwrap();
			self.stdout.write_all(b"\n").unwrap();
		} else {
			self.stdout.write_all(b"Error\n").unwrap();
		}

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

		match self.version_control.run_action(
			&action.name[..],
			|| String::from(""),
			|output| match output {
				Ok(output) => {
					write!(self.stdout, "{}\n\n", output).unwrap();
					write!(self.stdout, "done\n\n").unwrap();
				}
				Err(error) => {
					write!(self.stdout, "{}\n\n", error).unwrap();
					write!(self.stdout, "error\n\n").unwrap();
				}
			},
		) {
			Ok(_) => (),
			Err(error) => {
				write!(self.stdout, "{}\n\nerror\n\n", error).unwrap();
			}
		};
	}
}
