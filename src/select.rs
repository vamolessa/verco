use crossterm::*;

const RESET_COLOR: Attribute = Attribute::Reset;
const HELP_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 255,
	g: 180,
	b: 100,
});
const UNTRACKED_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 100,
	g: 180,
	b: 255,
});
const UNMODIFIED_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 255,
	g: 255,
	b: 255,
});
const MODIFIED_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 255,
	g: 200,
	b: 0,
});
const ADDED_COLOR: Colored = Colored::Fg(Color::Rgb { r: 0, g: 255, b: 0 });
const DELETED_COLOR: Colored = Colored::Fg(Color::Rgb { r: 255, g: 0, b: 0 });
const RENAMED_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 100,
	g: 100,
	b: 255,
});
const COPIED_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 255,
	g: 0,
	b: 255,
});
const UNMERGED_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 255,
	g: 180,
	b: 100,
});
const MISSING_COLOR: Colored = Colored::Fg(Color::Rgb { r: 255, g: 0, b: 0 });
const IGNORED_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 255,
	g: 180,
	b: 0,
});
const CLEAN_COLOR: Colored = Colored::Fg(Color::Rgb {
	r: 100,
	g: 180,
	b: 255,
});

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

impl State {
	fn color(&self) -> Colored {
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

pub fn select(
	terminal: &mut Terminal,
	cursor: &mut TerminalCursor,
	input: &mut TerminalInput,
	entries: &mut Vec<Entry>,
) -> bool {
	if entries.len() == 0 {
		return false;
	}

	print!(
		"{}{}j/k{} move, {}space{} (de)select, {}a{} (de)select all, {}c{} continue, {}ctrl+c{} cancel \n\n",
		RESET_COLOR,
		HELP_COLOR,
		RESET_COLOR,
		HELP_COLOR,
		RESET_COLOR,
		HELP_COLOR,
		RESET_COLOR,
		HELP_COLOR,
		RESET_COLOR,
		HELP_COLOR,
		RESET_COLOR,
	);

	cursor.hide().unwrap();
	cursor.save_position().unwrap();

	for e in entries.iter() {
		println!(
			"    {}{:?}{}\t{}",
			e.state.color(),
			e.state,
			RESET_COLOR,
			e.filename,
		);
	}

	let mut index = 0;
	let terminal_size = terminal.terminal_size();
	let selected;

	for i in 0..entries.len() {
		draw_entry_state(cursor, entries, i, i == index);
	}

	loop {
		//cursor.goto(terminal_size.0, terminal_size.1).unwrap();
		cursor.goto(0, terminal_size.1).unwrap();
		match input.read_char() {
			Ok(key) => {
				terminal.clear(ClearType::CurrentLine).unwrap();
				cursor.move_left(1);

				if key as u8 == 13 {
					println!("ENTER!");
				}

				if key.is_control() {
					const CTRL_C: char = 3u8 as char;
					if key == CTRL_C {
						selected = false;
						break;
					}
				} else {
					match key {
						'c' => {
							selected = entries.iter().any(|e| e.selected);
							break;
						}
						'j' => {
							draw_entry_state(cursor, entries, index, false);
							index = (index + 1) % entries.len();
							draw_entry_state(cursor, entries, index, true);
						}
						'k' => {
							draw_entry_state(cursor, entries, index, false);
							index = (index + entries.len() - 1) % entries.len();
							draw_entry_state(cursor, entries, index, true);
						}
						' ' => {
							entries[index].selected = !entries[index].selected;
							draw_entry_state(cursor, entries, index, true);
						}
						'a' => {
							let all_selected = entries.iter().all(|e| e.selected);
							for e in entries.iter_mut() {
								e.selected = !all_selected;
							}
							for i in 0..entries.len() {
								draw_entry_state(cursor, entries, i, i == index);
							}
						}
						_ => (),
					};
				}
			}
			Err(_) => {
				selected = false;
				break;
			}
		}
	}

	cursor.reset_position().unwrap();
	cursor.move_down(entries.len() as u16);
	cursor.show().unwrap();
	selected
}

fn draw_entry_state(
	cursor: &mut TerminalCursor,
	entries: &Vec<Entry>,
	index: usize,
	cursor_on: bool,
) {
	cursor.reset_position().unwrap();
	if index > 0 {
		cursor.move_down(index as u16);
	}

	let cursor_char = if cursor_on { '>' } else { ' ' };
	let select_char = if entries[index].selected { '+' } else { ' ' };
	print!("{}{} {}", RESET_COLOR, cursor_char, select_char);
}
