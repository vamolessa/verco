use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{self, Clear, ClearType},
    QueueableCommand, Result,
};

use std::io::Write;

use crate::{ctrlc_handler::CtrlcHandler, input, tui_util::ENTRY_COLOR};

const SELECTED_BG_COLOR: Color = Color::DarkGrey;
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

struct Select<'a, 'b, W>
where
    W: Write,
{
    write: &'a mut W,
    entries: &'b mut Vec<Entry>,
    index: usize,
    cursor_offset: (u16, u16),
}

impl<'a, 'b, W> Select<'a, 'b, W>
where
    W: Write,
{
    pub fn move_cursor(&mut self, delta: i32) -> Result<()> {
        let previous_cursor = self.index;
        let len = self.entries.len() as i32;
        self.index = ((self.index as i32 + delta + len) % len) as usize;
        let terminal_size = terminal::size()?;
        self.draw_entry(previous_cursor, terminal_size)?;
        self.draw_entry(self.index, terminal_size)?;
        Ok(())
    }

    pub fn draw_entry(&mut self, index: usize, terminal_size: (u16, u16)) -> Result<()> {
        self.write.queue(cursor::MoveTo(
            self.cursor_offset.0,
            self.cursor_offset.1 + index as u16,
        ))?;

        if index == self.index {
            self.write.queue(SetBackgroundColor(SELECTED_BG_COLOR))?;
        } else {
            self.write.queue(ResetColor)?;
        }

        let select_char = if self.entries[index].selected {
            '+'
        } else {
            ' '
        };

        let entry = &self.entries[index];
        let state_name = format!("{:?}", entry.state);
        self.write
            .queue(Print(select_char))?
            .queue(Print(' '))?
            .queue(SetForegroundColor(entry.state.color()))?
            .queue(Print(&state_name))?
            .queue(ResetColor)?;

        if index == self.index {
            self.write.queue(SetBackgroundColor(SELECTED_BG_COLOR))?;
        } else {
            self.write.queue(ResetColor)?;
        }

        let cursor_x = self.cursor_offset.0 + 2 + state_name.len() as u16;
        for _ in cursor_x..ITEM_NAME_COLUMN {
            self.write.queue(Print(' '))?;
        }
        let max_len = (entry.filename.len() as u16).min(terminal_size.0 - ITEM_NAME_COLUMN);
        let cursor_x = ITEM_NAME_COLUMN + max_len;
        self.write.queue(Print(
            &entry.filename[(entry.filename.len() - max_len as usize)..],
        ))?;
        for _ in cursor_x..terminal_size.0 {
            self.write.queue(Print(' '))?;
        }
        self.write.queue(ResetColor)?;
        Ok(())
    }
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
        SetForegroundColor(ENTRY_COLOR),
        SetAttribute(Attribute::Bold),
        Print("ctrl+j/ctrl+k"),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(ENTRY_COLOR),
        Print(" move, "),
        SetAttribute(Attribute::Bold),
        Print("space"),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(ENTRY_COLOR),
        Print(" (de)select, "),
        SetAttribute(Attribute::Bold),
        Print("ctrl+a"),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(ENTRY_COLOR),
        Print(" (de)select all, "),
        SetAttribute(Attribute::Bold),
        Print("enter"),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(ENTRY_COLOR),
        Print(" continue, "),
        SetAttribute(Attribute::Bold),
        Print("ctrl+c"),
        SetAttribute(Attribute::Reset),
        SetForegroundColor(ENTRY_COLOR),
        Print(" cancel\n"),
        cursor::Hide
    )?;
    write.flush()?;

    let mut select = Select {
        entries,
        write,
        index: 0,
        cursor_offset: cursor::position()?,
    };

    let selected;

    let terminal_size = terminal::size()?;
    for i in 0..select.entries.len() {
        select.draw_entry(i, terminal_size)?;
    }

    loop {
        select.write.queue(cursor::MoveTo(
            select.cursor_offset.0,
            select.cursor_offset.1,
        ))?;
        select.write.flush()?;
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
                modifiers: KeyModifiers::CONTROL,
            } => {
                selected = false;
                break;
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                selected = select.entries.iter().any(|e| e.selected);
                break;
            }
            KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::Down,
                ..
            } => {
                select.move_cursor(1)?;
            }
            KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::Up, ..
            } => {
                select.move_cursor(-1)?;
            }
            KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::PageDown,
                ..
            } => {
                select.move_cursor(1)?;
            }
            KeyEvent {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::PageUp,
                ..
            } => {
                select.move_cursor(-1)?;
            }
            KeyEvent {
                code: KeyCode::Char(' '),
                ..
            } => {
                select.entries[select.index].selected = !select.entries[select.index].selected;
                select.draw_entry(select.index, terminal_size)?;
            }
            KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                let all_selected = select.entries.iter().all(|e| e.selected);
                for e in select.entries.iter_mut() {
                    e.selected = !all_selected;
                }
                for i in 0..select.entries.len() {
                    select.draw_entry(select.index, terminal_size)?;
                }
            }
            _ => (),
        }
    }

    select.write.queue(cursor::Show)?;
    Ok(selected)
}
