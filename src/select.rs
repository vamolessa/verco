use crossterm::{
    cursor, execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    Result,
};

use std::io::Write;

use crate::input;

const HELP_COLOR: Color = Color::Rgb {
    r: 255,
    g: 180,
    b: 100,
};
const UNTRACKED_COLOR: Color = Color::Rgb {
    r: 100,
    g: 180,
    b: 255,
};
const UNMODIFIED_COLOR: Color = Color::Rgb {
    r: 255,
    g: 255,
    b: 255,
};
const MODIFIED_COLOR: Color = Color::Rgb {
    r: 255,
    g: 200,
    b: 0,
};
const ADDED_COLOR: Color = Color::Rgb { r: 0, g: 255, b: 0 };
const DELETED_COLOR: Color = Color::Rgb { r: 255, g: 0, b: 0 };
const RENAMED_COLOR: Color = Color::Rgb {
    r: 100,
    g: 100,
    b: 255,
};
const COPIED_COLOR: Color = Color::Rgb {
    r: 255,
    g: 0,
    b: 255,
};
const UNMERGED_COLOR: Color = Color::Rgb {
    r: 255,
    g: 180,
    b: 100,
};
const MISSING_COLOR: Color = Color::Rgb { r: 255, g: 0, b: 0 };
const IGNORED_COLOR: Color = Color::Rgb {
    r: 255,
    g: 180,
    b: 0,
};
const CLEAN_COLOR: Color = Color::Rgb {
    r: 100,
    g: 180,
    b: 255,
};

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

pub fn select<W>(stdout: &mut W, entries: &mut Vec<Entry>) -> Result<bool>
where
    W: Write,
{
    if entries.len() == 0 {
        return Ok(false);
    }

    queue!(
        stdout,
        ResetColor,
        SetForegroundColor(HELP_COLOR),
        Print("j/k"),
        ResetColor,
        Print(" move, "),
        SetForegroundColor(HELP_COLOR),
        Print("space"),
        ResetColor,
        Print(" (de)select, "),
        SetForegroundColor(HELP_COLOR),
        Print("a"),
        ResetColor,
        Print(" (de)select all, "),
        SetForegroundColor(HELP_COLOR),
        Print("c"),
        ResetColor,
        Print(" continue, "),
        SetForegroundColor(HELP_COLOR),
        Print("ctrl+c"),
        ResetColor,
        Print(" cancel\n\n"),
        cursor::Hide,
        cursor::SavePosition
    )?;

    for e in entries.iter() {
        queue!(
            stdout,
            Print("    "),
            SetForegroundColor(e.state.color()),
            Print(format!("{:?}", e.state)),
            ResetColor,
            Print('\t'),
            Print(&e.filename),
            Print('\n')
        )?;
    }

    let mut index = 0;
    let terminal_size = terminal::size()?;
    let selected;

    for i in 0..entries.len() {
        draw_entry_state(stdout, entries, i, i == index)?;
    }

    loop {
        execute!(stdout, cursor::MoveTo(terminal_size.0, terminal_size.1))?;
        match input::read_char() {
            Ok(key) => {
                queue!(
                    stdout,
                    Clear(ClearType::CurrentLine),
                    cursor::MoveTo(terminal_size.0, terminal_size.1)
                )?;

                match key {
                    // q or ctrl+c or esc
                    'q' | '\x03' | '\x1b' => {
                        selected = false;
                        break;
                    }
                    // enter
                    'c' | '\x0d' => {
                        selected = entries.iter().any(|e| e.selected);
                        break;
                    }
                    'j' | 'P' => {
                        draw_entry_state(stdout, entries, index, false)?;
                        index = (index + 1) % entries.len();
                        draw_entry_state(stdout, entries, index, true)?;
                    }
                    'k' | 'H' => {
                        draw_entry_state(stdout, entries, index, false)?;
                        index = (index + entries.len() - 1) % entries.len();
                        draw_entry_state(stdout, entries, index, true)?;
                    }
                    ' ' => {
                        entries[index].selected = !entries[index].selected;
                        draw_entry_state(stdout, entries, index, true)?;
                    }
                    'a' => {
                        let all_selected = entries.iter().all(|e| e.selected);
                        for e in entries.iter_mut() {
                            e.selected = !all_selected;
                        }
                        for i in 0..entries.len() {
                            draw_entry_state(stdout, entries, i, i == index)?;
                        }
                    }
                    _ => (),
                };
            }
            Err(_error) => (),
        }
    }

    queue!(
        stdout,
        cursor::RestorePosition,
        cursor::MoveDown(entries.len() as u16),
        cursor::Show
    )?;
    Ok(selected)
}

fn draw_entry_state<W>(
    stdout: &mut W,
    entries: &Vec<Entry>,
    index: usize,
    cursor_on: bool,
) -> Result<()>
where
    W: Write,
{
    queue!(stdout, cursor::RestorePosition)?;
    if index > 0 {
        queue!(stdout, cursor::MoveDown(index as u16))?;
    }

    let cursor_char = if cursor_on { '>' } else { ' ' };
    let select_char = if entries[index].selected { '+' } else { ' ' };

    queue!(
        stdout,
        ResetColor,
        Print(cursor_char),
        Print(' '),
        Print(select_char)
    )?;
    Ok(())
}
