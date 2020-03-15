use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    QueueableCommand, Result,
};

use std::io::Write;

use crate::{ctrlc_handler::CtrlcHandler, input};

const SELECTED_BG_COLOR: Color = Color::DarkGrey;
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
const ITEM_NAME_COLUMN: u16 = 16;

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

pub fn select<W>(
    write: &mut W,
    ctrlc_handler: &mut CtrlcHandler,
    entries: &mut Vec<Entry>,
) -> Result<bool>
where
    W: Write,
{
    if entries.len() == 0 {
        return Ok(false);
    }

    queue!(
        write,
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
        Print("ctrl+a"),
        ResetColor,
        Print(" (de)select all, "),
        SetForegroundColor(HELP_COLOR),
        Print("enter"),
        ResetColor,
        Print(" continue, "),
        SetForegroundColor(HELP_COLOR),
        Print("ctrl+c"),
        ResetColor,
        Print(" cancel\n\n"),
        cursor::Hide
    )?;

    let mut index = 0;
    let terminal_size = terminal::size()?;
    let cursor_position = cursor::position()?;
    let selected;

    for i in 0..entries.len() {
        draw_entry(write, entries, i, i == index, cursor_position)?;
    }

    loop {
        queue!(
            write,
            cursor::MoveTo(terminal_size.0 - 2, terminal_size.1 - 2),
            Clear(ClearType::CurrentLine)
        )?;
        write.flush()?;
        match input::read_key(ctrlc_handler)? {
            KeyEvent {
                code: KeyCode::Esc, ..
            }
            | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::Char('q'),
                ..
            } => {
                selected = false;
                break;
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                selected = entries.iter().any(|e| e.selected);
                break;
            }
            KeyEvent {
                code: KeyCode::Char('j'),
                ..
            }
            | KeyEvent {
                code: KeyCode::Down,
                ..
            } => {
                draw_entry(write, entries, index, false, cursor_position)?;
                index = (index + 1) % entries.len();
                draw_entry(write, entries, index, true, cursor_position)?;
            }
            KeyEvent {
                code: KeyCode::Char('k'),
                ..
            }
            | KeyEvent {
                code: KeyCode::Up, ..
            } => {
                draw_entry(write, entries, index, false, cursor_position)?;
                index = (index + entries.len() - 1) % entries.len();
                draw_entry(write, entries, index, true, cursor_position)?;
            }
            KeyEvent {
                code: KeyCode::Char(' '),
                ..
            } => {
                entries[index].selected = !entries[index].selected;
                draw_entry(write, entries, index, true, cursor_position)?;
            }
            KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                let all_selected = entries.iter().all(|e| e.selected);
                for e in entries.iter_mut() {
                    e.selected = !all_selected;
                }
                for i in 0..entries.len() {
                    draw_entry(write, entries, i, i == index, cursor_position)?;
                }
            }
            _ => (),
        }
    }

    queue!(
        write,
        cursor::MoveTo(cursor_position.0, cursor_position.1),
        cursor::MoveDown(entries.len() as u16),
        cursor::Show
    )?;
    Ok(selected)
}

fn draw_entry<W>(
    write: &mut W,
    entries: &Vec<Entry>,
    index: usize,
    cursor_on: bool,
    cursor_offset: (u16, u16),
) -> Result<()>
where
    W: Write,
{
    write.queue(cursor::MoveTo(
        cursor_offset.0,
        cursor_offset.1 + index as u16,
    ))?;

    if cursor_on {
        write.queue(SetBackgroundColor(SELECTED_BG_COLOR))?;
    } else {
        write.queue(ResetColor)?;
    }

    let cursor_char = if cursor_on { '>' } else { ' ' };
    let select_char = if entries[index].selected { '+' } else { ' ' };

    let entry = &entries[index];
    let state_name = format!("{:?}", entry.state);
    queue!(
        write,
        Print(cursor_char),
        Print(' '),
        Print(select_char),
        Print(' '),
        SetForegroundColor(entry.state.color()),
        Print(&state_name),
        ResetColor
    )?;

    if cursor_on {
        write.queue(SetBackgroundColor(SELECTED_BG_COLOR))?;
    } else {
        write.queue(ResetColor)?;
    }

    let cursor_x = cursor_offset.0 + 4 + state_name.len() as u16;
    for _ in cursor_x..ITEM_NAME_COLUMN {
        write.queue(Print(' '))?;
    }
    let cursor_x = ITEM_NAME_COLUMN + entry.filename.len() as u16;
    write.queue(Print(&entry.filename))?;
    for _ in cursor_x..terminal::size()?.0 - 1 {
        write.queue(Print(' '))?;
    }
    write.queue(ResetColor)?;
    Ok(())
}
