use crossterm::{
    cursor,
    event::{self, KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor,
        SetForegroundColor,
    },
    terminal::{Clear, ClearType},
    QueueableCommand, Result,
};

use std::io::Write;

use crate::{
    input,
    tui_util::{
        move_cursor, AvailableSize, TerminalSize, ENTRY_COLOR,
        SELECTED_BG_COLOR,
    },
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
const ITEM_NAME_COLUMN: usize = 16;

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

struct Select<'a> {
    entries: &'a mut Vec<Entry>,
    scroll: usize,
    cursor: usize,
    header_position: (u16, u16),
    filter: Vec<char>,
}

impl<'a> Select<'a> {
    fn filtered_entries(&self) -> impl Iterator<Item = &Entry> {
        let filter = &self.filter;
        let filter_len = filter.len();
        self.entries.iter().filter(move |e| {
            let mut filter_index = 0;
            for c in e.filename.chars() {
                if filter_index >= filter_len {
                    break;
                }

                if filter[filter_index] == c {
                    filter_index += 1;
                }
            }

            filter_index >= filter_len
        })
    }

    fn filtered_entries_mut(&mut self) -> impl Iterator<Item = &mut Entry> {
        let filter = &self.filter;
        let filter_len = filter.len();
        self.entries.iter_mut().filter(move |e| {
            let mut filter_index = 0;
            for c in e.filename.chars() {
                if filter_index >= filter_len {
                    break;
                }

                if filter[filter_index] == c {
                    filter_index += 1;
                }
            }

            filter_index >= filter_len
        })
    }

    fn move_cursor<W>(
        &mut self,
        write: &mut W,
        available_size: AvailableSize,
        delta: i32,
    ) -> Result<()>
    where
        W: Write,
    {
        let entry_count = self.filtered_entries().count();
        move_cursor(
            &mut self.scroll,
            &mut self.cursor,
            available_size,
            entry_count,
            delta,
        );

        self.draw_all_entries(write, available_size)?;

        Ok(())
    }

    fn draw_all_entries<W>(
        &self,
        write: &mut W,
        available_size: AvailableSize,
    ) -> Result<()>
    where
        W: Write,
    {
        queue!(
            write,
            Clear(ClearType::FromCursorDown),
            cursor::MoveTo(0, 2)
        )?;

        let end_index = self
            .filtered_entries()
            .count()
            .min(self.scroll + available_size.height);
        for i in self.scroll..end_index {
            self.draw_entry(write, i, available_size)?;
        }
        Ok(())
    }

    fn draw_entry<W>(
        &self,
        write: &mut W,
        index: usize,
        available_size: AvailableSize,
    ) -> Result<()>
    where
        W: Write,
    {
        write
            .queue(cursor::MoveTo(0, 2 + index as u16 - self.scroll as u16))?;

        if index == self.cursor {
            write.queue(SetBackgroundColor(SELECTED_BG_COLOR))?;
        } else {
            write.queue(ResetColor)?;
        }

        let entry = match self.filtered_entries().nth(index) {
            Some(entry) => entry,
            None => return Ok(()),
        };

        let select_char = if entry.selected { '+' } else { ' ' };
        let state_name = format!("{:?}", entry.state);

        write
            .queue(Print(select_char))?
            .queue(Print(' '))?
            .queue(SetForegroundColor(entry.state.color()))?
            .queue(Print(&state_name))?
            .queue(ResetColor)?;

        if index == self.cursor {
            write.queue(SetBackgroundColor(SELECTED_BG_COLOR))?;
        } else {
            write.queue(ResetColor)?;
        }

        let cursor_x = 2 + state_name.len();
        for _ in cursor_x..ITEM_NAME_COLUMN {
            write.queue(Print(' '))?;
        }
        let max_len =
            (entry.filename.len()).min(available_size.width - ITEM_NAME_COLUMN);
        let cursor_x = ITEM_NAME_COLUMN + max_len;
        write.queue(Print(
            &entry.filename[(entry.filename.len() - max_len as usize)..],
        ))?;
        for _ in cursor_x..available_size.width {
            write.queue(Print(' '))?;
        }
        write.queue(ResetColor)?;
        Ok(())
    }

    fn draw_header<W>(
        &self,
        write: &mut W,
        available_size: AvailableSize,
    ) -> Result<()>
    where
        W: Write,
    {
        let width = available_size.width as u16;
        let filter_text = "filter: ";
        let filter_len = filter_text.len() + self.filter.len();

        queue!(
            write,
            cursor::MoveTo(self.header_position.0, self.header_position.1),
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
            Clear(ClearType::UntilNewLine),
            cursor::MoveToColumn(width - width.min(filter_len as u16)),
        )?;

        if self.filter.len() > 0 {
            write.queue(Print(filter_text))?;
        }

        for c in &self.filter {
            write.queue(Print(c))?;
        }
        queue!(write, cursor::MoveToNextLine(1), ResetColor)?;

        Ok(())
    }
}

pub fn select<W>(write: &mut W, entries: &mut Vec<Entry>) -> Result<bool>
where
    W: Write,
{
    if entries.len() == 0 {
        return Ok(false);
    }

    write.flush()?;

    let mut select = Select {
        entries,
        scroll: 0,
        cursor: 0,
        header_position: cursor::position()?,
        filter: Vec::new(),
    };

    let mut available_size =
        AvailableSize::from_temrinal_size(TerminalSize::get()?);
    select.draw_header(write, available_size)?;
    select.draw_all_entries(write, available_size)?;

    let selected;
    loop {
        write.queue(cursor::MoveTo(0, 2))?;
        write.flush()?;
        match event::read()? {
            event::Event::Resize(width, height) => {
                available_size =
                    AvailableSize::from_temrinal_size(TerminalSize {
                        width,
                        height,
                    });
            }
            event::Event::Key(key_event) => match key_event {
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
                    if select.filter.len() > 0 {
                        select.filter.clear();
                        on_filter_changed(&mut select, write, available_size)?;
                    } else {
                        selected = false;
                        break;
                    }
                }
                KeyEvent {
                    code: KeyCode::Enter,
                    ..
                } => {
                    selected = select.filtered_entries().any(|e| e.selected);
                    break;
                }
                KeyEvent {
                    code: KeyCode::Char('j'),
                    modifiers: KeyModifiers::CONTROL,
                }
                | KeyEvent {
                    code: KeyCode::Char('n'),
                    modifiers: KeyModifiers::CONTROL,
                }
                | KeyEvent {
                    code: KeyCode::Down,
                    ..
                } => {
                    select.move_cursor(write, available_size, 1)?;
                }
                KeyEvent {
                    code: KeyCode::Char('k'),
                    modifiers: KeyModifiers::CONTROL,
                }
                | KeyEvent {
                    code: KeyCode::Char('p'),
                    modifiers: KeyModifiers::CONTROL,
                }
                | KeyEvent {
                    code: KeyCode::Up, ..
                } => {
                    select.move_cursor(write, available_size, -1)?;
                }
                KeyEvent {
                    code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::CONTROL,
                }
                | KeyEvent {
                    code: KeyCode::PageDown,
                    ..
                } => {
                    let height = select
                        .filtered_entries()
                        .count()
                        .min(available_size.height);
                    select.move_cursor(
                        write,
                        available_size,
                        height as i32 / 2,
                    )?;
                }
                KeyEvent {
                    code: KeyCode::Char('u'),
                    modifiers: KeyModifiers::CONTROL,
                }
                | KeyEvent {
                    code: KeyCode::PageUp,
                    ..
                } => {
                    let height = select
                        .filtered_entries()
                        .count()
                        .min(available_size.height);
                    select.move_cursor(
                        write,
                        available_size,
                        height as i32 / -2,
                    )?;
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
                    select.draw_all_entries(write, available_size)?;
                }
                KeyEvent {
                    code: KeyCode::Char('e'),
                    modifiers: KeyModifiers::CONTROL,
                }
                | KeyEvent {
                    code: KeyCode::End, ..
                } => {
                    let entries_len = select.filtered_entries().count();
                    select.scroll = 0
                        .max(entries_len as i32 - available_size.height as i32)
                        as usize;
                    select.cursor = entries_len - 1;
                    select.draw_all_entries(write, available_size)?;
                }
                KeyEvent {
                    code: KeyCode::Char(' '),
                    ..
                } => {
                    let cursor = select.cursor;
                    if let Some(e) = select.filtered_entries_mut().nth(cursor) {
                        e.selected = !e.selected;
                    }
                    select.draw_entry(write, select.cursor, available_size)?;
                }
                KeyEvent {
                    code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    let all_selected =
                        select.filtered_entries().all(|e| e.selected);
                    for e in select.filtered_entries_mut() {
                        e.selected = !all_selected;
                    }
                    select.draw_all_entries(write, available_size)?;
                }
                KeyEvent {
                    code: KeyCode::Char('h'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    select.filter.swap_remove(select.filter.len() - 1);
                    on_filter_changed(&mut select, write, available_size)?;
                }
                KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    select.filter.clear();
                    on_filter_changed(&mut select, write, available_size)?;
                }
                KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers,
                } => {
                    if modifiers == KeyModifiers::CONTROL {
                        select.filter.clear();
                    } else if select.filter.len() > 0 {
                        select.filter.swap_remove(select.filter.len() - 1);
                    }
                    on_filter_changed(&mut select, write, available_size)?;
                }
                key_event => {
                    if let Some(c) = input::key_to_char(key_event) {
                        select.filter.push(c);
                        on_filter_changed(&mut select, write, available_size)?;
                    }
                }
            },
            _ => (),
        }
    }

    Ok(selected)
}

fn on_filter_changed<W>(
    select: &mut Select,
    write: &mut W,
    available_size: AvailableSize,
) -> Result<()>
where
    W: Write,
{
    select.cursor = 0;
    select.scroll = 0;
    select.draw_header(write, available_size)?;
    select.draw_all_entries(write, available_size)?;
    Ok(())
}
