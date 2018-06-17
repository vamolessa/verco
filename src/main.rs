extern crate rustyline;
extern crate termion;

use std::env;
use std::path::PathBuf;

mod git_actions;
mod hg_actions;
mod tui;
mod version_control_actions;

use git_actions::GitActions;
use hg_actions::HgActions;

fn main() {
	test();

	let current_dir_path = env::current_dir().unwrap();
	let current_dir = current_dir_path.to_str().unwrap();

	if subdir_exists(&current_dir_path, ".git") {
		let actions = GitActions {
			current_dir: &current_dir,
		};
		tui::show_tui(&current_dir, &actions);
	} else if subdir_exists(&current_dir_path, ".hg") {
		let actions = HgActions {
			current_dir: &current_dir,
		};
		tui::show_tui(&current_dir, &actions);
	} else {
		println!("no repository found");
	}
}

fn subdir_exists(basedir: &PathBuf, subdir: &str) -> bool {
	let mut path = basedir.clone();
	path.push(subdir);
	path.exists()
}

use std::io::{stdin, stdout, BufRead, Write};
use termion::color;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

struct S {
	pub name: &'static str,
	pub selected: bool,
}

fn test() {
	let _guard = termion::init();

	let stdin = stdin();
	let mut stdin = stdin.lock();
	let mut stdout = stdout().into_raw_mode().unwrap();

	let mut index: usize = 0;
	let mut vec = Vec::new();

	vec.push(S {
		name: "thing 1",
		selected: false,
	});
	vec.push(S {
		name: "thing 2",
		selected: false,
	});
	vec.push(S {
		name: "thing 3",
		selected: false,
	});

	loop {
		write!(stdout, "{}", termion::clear::All).unwrap();

		for e in &vec {
			let selection = if e.selected { "*" } else { " " };
			write!(stdout, " {} {}\n", selection, e.name).unwrap();
		}

		let y = (index + 1) as u16;
		write!(stdout, "{}", termion::cursor::Goto(1, y)).unwrap();
		stdout.flush().unwrap();

		let key = (&mut stdin).keys().next().unwrap().unwrap();

		match key {
			Key::Ctrl('c') => break,
			Key::Char('j') => index = (index + 1) % vec.len(),
			Key::Char('k') => index = (index + vec.len() - 1) % vec.len(),
			Key::Char('a') => vec[index].selected = !vec[index].selected,
			_ => (),
		}
	}
}
