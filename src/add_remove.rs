use std::io::{BufRead, Write};
use termion::color;
use termion::event::Key;
use termion::input::TermRead;

#[derive(Clone, Debug)]
pub enum State {
	Untracked,
	Unmodified,
	Modified,
	Added,
	Deleted,
	Renamed,
	Copied,
	Unmerged,
}

const RESET_COLOR: color::Fg<color::Reset> = color::Fg(color::Reset);
const RESET_BG_COLOR: color::Bg<color::Reset> = color::Bg(color::Reset);

const UNTRACKED_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(100, 180, 255));
const UNMODIFIED_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 255, 255));
const MODIFIED_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 200, 0));
const ADDED_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(0, 255, 0));
const DELETED_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 0, 0));
const RENAMED_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(100, 100, 255));
const COPIED_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 0, 255));
const UNMERGED_COLOR: color::Fg<color::Rgb> = color::Fg(color::Rgb(255, 180, 100));

impl State {
	fn color(&self) -> color::Fg<color::Rgb> {
		match self {
			State::Untracked => UNTRACKED_COLOR,
			State::Unmodified => UNMODIFIED_COLOR,
			State::Modified => MODIFIED_COLOR,
			State::Added => ADDED_COLOR,
			State::Deleted => DELETED_COLOR,
			State::Renamed => RENAMED_COLOR,
			State::Copied => COPIED_COLOR,
			State::Unmerged => UNMERGED_COLOR,
		}
	}
}

#[derive(Clone)]
pub struct Entry {
	pub filename: String,
	pub selected: bool,
	pub state: State,
}

pub fn draw_add_remove_selection<R: BufRead, W: Write>(
	stdin: &mut R,
	stdout: &mut W,
	entries: &mut Vec<Entry>,
	cursor_index: &mut usize,
) -> bool {
	write!(
		stdout,
		"{}(j/k) move, space (de)select, a (de)select all, enter continues\n\n",
		RESET_BG_COLOR
	).unwrap();

	let mut index = *cursor_index;

	for (i, e) in entries.iter().enumerate() {
		let cursor = if i == index { ">" } else { " " };
		let selection = if e.selected { "+" } else { " " };
		write!(
			stdout,
			"{}{} {} {}{:?}\t{}{}\n",
			RESET_COLOR,
			cursor,
			selection,
			e.state.color(),
			e.state,
			RESET_COLOR,
			e.filename
		).unwrap();
	}

	stdout.flush().unwrap();

	let key = stdin.keys().next().unwrap().unwrap();

	match key {
		Key::Char('\n') => return false,
		Key::Ctrl('c') => return false,
		Key::Char('j') => index = (index + 1) % entries.len(),
		Key::Char('k') => index = (index + entries.len() - 1) % entries.len(),
		Key::Char(' ') => entries[index].selected = !entries[index].selected,
		Key::Char('a') => if let Some(first) = entries.first().cloned() {
			for e in entries.iter_mut() {
				e.selected = !first.selected;
			}
		},
		_ => (),
	};

	*cursor_index = index;
	true
}
