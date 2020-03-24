use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal, QueueableCommand, Result,
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
    scroll: usize,
    cursor: usize,
    cursor_offset: (u16, u16),
}

impl<'a, 'b, W> Select<'a, 'b, W>
where
    W: Write,
{
    fn available_size(&self) -> (u16, u16) {
        let size = terminal::size().unwrap_or((0, 0));
        (
            size.0.max(self.cursor_offset.0) - self.cursor_offset.0,
            size.1.max(self.cursor_offset.1) - self.cursor_offset.1,
        )
    }

    fn move_cursor(&mut self, delta: i32) -> Result<()> {
        let previous_cursor = self.cursor;
        let len = self.entries.len();
        let target_cursor = self.cursor as i32 + delta;
        self.cursor = if target_cursor < 0 {
            if previous_cursor == 0 {
                (target_cursor + len as i32) as usize % len
            } else {
                0
            }
        } else if target_cursor >= len as i32 {
            if previous_cursor == len - 1 {
                (target_cursor + len as i32) as usize % len
            } else {
                len - 1
            }
        } else {
            target_cursor as usize
        };

        let available_size = self.available_size();

        if self.cursor < self.scroll {
            self.scroll = self.cursor;
            self.draw_all(available_size)?;
        } else if self.cursor >= self.scroll + available_size.1 as usize {
            self.scroll = self.cursor - available_size.1 as usize + 1;
            self.draw_all(available_size)?;
        } else {
            self.draw_entry(previous_cursor, available_size)?;
            self.draw_entry(self.cursor, available_size)?;
        }

        Ok(())
    }

    fn draw_all(&mut self, available_size: (u16, u16)) -> Result<()> {
        let end_index = self
            .entries
            .len()
            .min(self.scroll + self.available_size().1 as usize);
        for i in self.scroll..end_index {
            self.draw_entry(i, available_size)?;
        }
        Ok(())
    }

    fn draw_entry(&mut self, index: usize, available_size: (u16, u16)) -> Result<()> {
        self.write.queue(cursor::MoveTo(
            self.cursor_offset.0,
            self.cursor_offset.1 + index as u16 - self.scroll as u16,
        ))?;

        if index == self.cursor {
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

        if index == self.cursor {
            self.write.queue(SetBackgroundColor(SELECTED_BG_COLOR))?;
        } else {
            self.write.queue(ResetColor)?;
        }

        let cursor_x = self.cursor_offset.0 + 2 + state_name.len() as u16;
        for _ in cursor_x..ITEM_NAME_COLUMN {
            self.write.queue(Print(' '))?;
        }
        let max_len = (entry.filename.len() as u16).min(available_size.0 - ITEM_NAME_COLUMN);
        let cursor_x = ITEM_NAME_COLUMN + max_len;
        self.write.queue(Print(
            &entry.filename[(entry.filename.len() - max_len as usize)..],
        ))?;
        for _ in cursor_x..available_size.0 {
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
        Print(" cancel"),
        cursor::MoveToNextLine(1),
    )?;
    write.flush()?;

    let mut select = Select {
        entries,
        write,
        scroll: 0,
        cursor: 0,
        cursor_offset: cursor::position()?,
    };

    let selected;

    let available_size = select.available_size();
    select.draw_all(available_size)?;

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
                let height = select.entries.len().min(available_size.1 as usize);
                select.move_cursor(height as i32 / 2)?;
            }
            KeyEvent {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::PageUp,
                ..
            } => {
                let height = select.entries.len().min(available_size.1 as usize);
                select.move_cursor(height as i32 / -2)?;
            }
            KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::Char('b'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::Home,
                ..
            } => {
                select.scroll = 0;
                select.cursor = 0;
                select.draw_all(available_size)?;
            }
            KeyEvent {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::End, ..
            } => {
                select.scroll =
                    0.max(select.entries.len() as i32 - select.available_size().1 as i32) as usize;
                select.cursor = select.entries.len() - 1;
                select.draw_all(available_size)?;
            }
            KeyEvent {
                code: KeyCode::Char(' '),
                ..
            } => {
                select.entries[select.cursor].selected = !select.entries[select.cursor].selected;
                select.draw_entry(select.cursor, available_size)?;
            }
            KeyEvent {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                let all_selected = select.entries.iter().all(|e| e.selected);
                for e in select.entries.iter_mut() {
                    e.selected = !all_selected;
                }
                select.draw_all(available_size)?;
            }
            _ => (),
        }
    }

    Ok(selected)
}
