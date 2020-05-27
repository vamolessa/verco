use crossterm::{
    cursor,
    event::{self, KeyCode, KeyEvent, KeyModifiers},
    handle_command,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
    QueueableCommand, Result,
};

use std::io::Write;

use crate::{
    input,
    tui_util::{
        draw_filter_bar, fuzzy_matches, move_cursor, AvailableSize,
        TerminalSize, SELECTED_BG_COLOR,
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
    entries: &'a mut [Entry],
    scroll: usize,
    cursor: usize,
    filter: Vec<char>,
}

impl<'a> Select<'a> {
    fn filtered_entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries
            .iter()
            .filter(move |e| fuzzy_matches(&e.filename[..], &self.filter[..]))
    }

    fn filtered_entries_mut(&mut self) -> impl Iterator<Item = &mut Entry> {
        let filter = &self.filter;
        self.entries
            .iter_mut()
            .filter(move |e| fuzzy_matches(&e.filename[..], &filter[..]))
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

        self.draw_all_entries(write, available_size)
    }

    fn draw_all_entries<W>(
        &self,
        write: &mut W,
        available_size: AvailableSize,
    ) -> Result<()>
    where
        W: Write,
    {
        handle_command!(write, cursor::MoveTo(0, 1))?;
        handle_command!(write, ResetColor)?;

        for (i, entry) in self
            .filtered_entries()
            .enumerate()
            .skip(self.scroll)
            .take(available_size.height)
        {
            if i == self.cursor {
                handle_command!(write, SetBackgroundColor(SELECTED_BG_COLOR))?;
            } else {
                handle_command!(write, ResetColor)?;
            }

            let select_char = if entry.selected { '+' } else { ' ' };
            let state_name = format!("{:?}", entry.state);

            handle_command!(write, Print(select_char))?;
            handle_command!(write, Print(' '))?;
            handle_command!(write, SetForegroundColor(entry.state.color()))?;
            handle_command!(write, Print(&state_name))?;
            handle_command!(write, ResetColor)?;

            if i == self.cursor {
                handle_command!(write, SetBackgroundColor(SELECTED_BG_COLOR))?;
            } else {
                handle_command!(write, ResetColor)?;
            }

            let cursor_x = 2 + state_name.len();
            for _ in cursor_x..ITEM_NAME_COLUMN {
                handle_command!(write, Print(' '))?;
            }
            let slice_start = entry
                .filename
                .char_indices()
                .rev()
                .take(available_size.width - ITEM_NAME_COLUMN)
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);

            handle_command!(write, Print(&entry.filename[slice_start..]))?;
            handle_command!(write, Clear(ClearType::UntilNewLine))?;
            handle_command!(write, cursor::MoveToNextLine(1))?;
        }

        handle_command!(write, ResetColor)?;
        handle_command!(write, Clear(ClearType::FromCursorDown))?;
        draw_filter_bar(write, &self.filter[..], false)?;

        Ok(())
    }

    fn on_filter_changed<W>(
        &mut self,
        write: &mut W,
        available_size: AvailableSize,
    ) -> Result<()>
    where
        W: Write,
    {
        self.cursor = 0;
        self.scroll = 0;
        self.draw_all_entries(write, available_size)?;
        Ok(())
    }
}

pub fn select<W>(write: &mut W, entries: &mut [Entry]) -> Result<bool>
where
    W: Write,
{
    if entries.len() == 0 {
        return Ok(false);
    }

    let mut select = Select {
        entries,
        scroll: 0,
        cursor: 0,
        filter: Vec::new(),
    };

    let mut available_size =
        AvailableSize::from_temrinal_size(TerminalSize::get()?);
    select.draw_all_entries(write, available_size)?;

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
                        select.on_filter_changed(write, available_size)?;
                    } else {
                        for e in select.filtered_entries_mut() {
                            e.selected = false;
                        }
                        break;
                    }
                }
                KeyEvent {
                    code: KeyCode::Enter,
                    ..
                } => {
                    let cursor = select.cursor;
                    if select.entries.iter().filter(|e| e.selected).count() == 0
                    {
                        if let Some(e) =
                            select.filtered_entries_mut().nth(cursor)
                        {
                            e.selected = true;
                        }
                    }
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
                    select.draw_all_entries(write, available_size)?;
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
                }
                | KeyEvent {
                    code: KeyCode::Backspace,
                    ..
                } => {
                    if select.filter.len() > 0 {
                        select.filter.remove(select.filter.len() - 1);
                    }
                    select.on_filter_changed(write, available_size)?;
                }
                KeyEvent {
                    code: KeyCode::Char('w'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    select.filter.clear();
                    select.on_filter_changed(write, available_size)?;
                }
                key_event => {
                    if let Some(c) = input::key_to_char(key_event) {
                        select.filter.push(c);
                        select.on_filter_changed(write, available_size)?;
                    }
                }
            },
            _ => (),
        }
    }

    Ok(select.entries.iter().filter(|e| e.selected).count() > 0)
}
