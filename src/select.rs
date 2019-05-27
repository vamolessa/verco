use std::io::{BufRead, Write};
use crossterm::*;

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
	Missing,
	Ignored,
	Clean,
}

const RESET_COLOR: Attribute = Attribute::Reset;
const RESET_BG_COLOR: Attribute = Attribute::Reset;

const HELP_COLOR: Color = Colored::Fg(Color::Rgb(255, 180, 100));

const UNTRACKED_COLOR: Color = Colored::Fg(Color::Rgb(100, 180, 255));
const UNMODIFIED_COLOR: Color = Colored::Fg(Color::Rgb(255, 255, 255));
const MODIFIED_COLOR: Color = Colored::Fg(Color::Rgb(255, 200, 0));
const ADDED_COLOR: Color = Colored::Fg(Color::Rgb(0, 255, 0));
const DELETED_COLOR: Color = Colored::Fg(Color::Rgb(255, 0, 0));
const RENAMED_COLOR: Color = Colored::Fg(Color::Rgb(100, 100, 255));
const COPIED_COLOR: Color = Colored::Fg(Color::Rgb(255, 0, 255));
const UNMERGED_COLOR: Color = Colored::Fg(Color::Rgb(255, 180, 100));
const MISSING_COLOR: Color = Colored::Fg(Color::Rgb(255, 0, 0));
const IGNORED_COLOR: Color = Colored::Fg(Color::Rgb(255, 180, 0));
const CLEAN_COLOR: Color = Colored::Fg(Color::Rgb(100, 180, 255));

impl State {
	fn color(&self) -> Color {
		match self {
			State::Untracked => UNTRACKED_COLOR,
			State::Unmodified => UNMODIFIED_COLOR,
			State::Modified => MODIFIED_COLOR,
			State::Added => ADDED_COLOR,
			State::Deleted => DELETED_COLOR,
			State::Renamed => RENAMED_COLOR,
			State::Copied => COPIED_COLOR,
			State::Unmerged => UNMERGED_COLOR,
			State::Missing => MISSING_COLOR,
			State::Ignored => IGNORED_COLOR,
			State::Clean => CLEAN_COLOR,
		}
	}
}

#[derive(Clone)]
pub struct Entry {
	pub filename: String,
	pub selected: bool,
	pub state: State,
}

pub fn draw_select(
	input: &mut TerminalInput,
	entries: &mut Vec<Entry>,
	cursor_index: &mut usize,
) -> bool {
	if entries.len() == 0 {
		return false;
	}

	print!(
		"{}{}j/k{} move, {}space{} (de)select, {}a{} (de)select all, {}enter{} continues\n\n",
		RESET_BG_COLOR,
		HELP_COLOR,
		RESET_COLOR,
		HELP_COLOR,
		RESET_COLOR,
		HELP_COLOR,
		RESET_COLOR,
		HELP_COLOR,
		RESET_COLOR,
	);

	let mut index = *cursor_index;

	for (i, e) in entries.iter().enumerate() {
		let cursor = if i == index { ">" } else { " " };
		let selection = if e.selected { "+" } else { " " };
		print!(
			"{}{} {} {}{:?}\t{}{}\n",
			RESET_COLOR,
			cursor,
			selection,
			e.state.color(),
			e.state,
			RESET_COLOR,
			e.filename
		);
	}

	//stdout.flush().unwrap();

	match input.read_char() {
		Some(key) => {
			match key {
				'\n' => return false,
				'q' => return false,
				'j' => index = (index + 1) % entries.len(),
				'k' => index = (index + entries.len() - 1) % entries.len(),
				' ' => entries[index].selected = !entries[index].selected,
				'a' => {
					if let Some(first) = entries.first().cloned() {
						for e in entries.iter_mut() {
							e.selected = !first.selected;
						}
					}
				}
				_ => (),
			};
		}
		Err(error) => {
			return false;
		}
	}

	*cursor_index = index;
	true
}
