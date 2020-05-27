use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    handle_command, queue,
    style::{Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
    Result,
};

use std::io::Write;

use crate::{
    action::ActionKind,
    input,
    tui_util::{
        fuzzy_matches, move_cursor, AvailableSize, TerminalSize, ENTRY_COLOR,
        SELECTED_BG_COLOR,
    },
};

pub struct ScrollView {
    action_kind: ActionKind,
    content: String,
    scroll: usize,
    cursor: Option<usize>,
    is_filtering: bool,
    filter: Vec<char>,
}

impl Default for ScrollView {
    fn default() -> Self {
        Self {
            action_kind: ActionKind::Quit,
            content: String::with_capacity(1024 * 4),
            scroll: 0,
            cursor: None,
            is_filtering: false,
            filter: Vec::new(),
        }
    }
}

impl ScrollView {
    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    pub fn set_content(
        &mut self,
        content: &str,
        action_kind: ActionKind,
        terminal_size: TerminalSize,
    ) {
        self.content.clear();
        self.content.push_str(content);

        self.is_filtering = false;
        self.filter.clear();

        if self.action_kind != action_kind {
            self.scroll = 0;
            self.cursor = if action_kind.can_select_output() {
                Some(0)
            } else {
                None
            };
        } else {
            self.scroll(AvailableSize::from_temrinal_size(terminal_size), 0);
        }

        self.action_kind = action_kind;
    }

    pub fn show<W>(
        &self,
        write: &mut W,
        terminal_size: TerminalSize,
    ) -> Result<()>
    where
        W: Write,
    {
        let line_formatter = self.action_kind.line_formatter();
        let available_size = AvailableSize::from_temrinal_size(terminal_size);

        let width = available_size.width as u16;
        let filter_text = "filter: ";
        let filter_len = filter_text.len() + self.filter.len();

        queue!(
            write,
            cursor::MoveTo(width - width.min(filter_len as u16), 1),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ENTRY_COLOR),
        )?;

        if self.is_filtering {
            handle_command!(write, Print(filter_text))?;
        }

        for c in &self.filter {
            handle_command!(write, Print(c))?;
        }

        queue!(write, cursor::MoveToNextLine(1), ResetColor)?;

        for (i, line) in self
            .filtered_lines()
            .enumerate()
            .skip(self.scroll)
            .take(available_size.height)
        {
            if let Some(cursor) = self.cursor {
                if cursor == i {
                    handle_command!(
                        write,
                        SetBackgroundColor(SELECTED_BG_COLOR)
                    )?;
                }

                line_formatter(write, line, available_size)?;
                handle_command!(write, Clear(ClearType::UntilNewLine))?;
                handle_command!(write, cursor::MoveToNextLine(1))?;
                handle_command!(write, ResetColor)?;
            } else {
                handle_command!(write, Clear(ClearType::CurrentLine))?;
                line_formatter(write, line, available_size)?;
                handle_command!(write, cursor::MoveToNextLine(1))?;
            }
        }

        handle_command!(write, Clear(ClearType::FromCursorDown))?;

        Ok(())
    }

    pub fn update<W>(
        &mut self,
        write: &mut W,
        key_event: KeyEvent,
        terminal_size: TerminalSize,
    ) -> Result<bool>
    where
        W: Write,
    {
        let available_size = AvailableSize::from_temrinal_size(terminal_size);
        match key_event {
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
            }
            | KeyEvent {
                code: KeyCode::Enter,
                ..
            }
            | KeyEvent {
                code: KeyCode::Char('\n'),
                ..
            } => {
                self.scroll(available_size, 1);
                self.show(write, terminal_size)?;
                Ok(true)
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
                self.scroll(available_size, -1);
                self.show(write, terminal_size)?;
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char('d'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::PageDown,
                ..
            } => {
                self.scroll(available_size, available_size.height as i32 / 2);
                self.show(write, terminal_size)?;
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::PageUp,
                ..
            } => {
                self.scroll(available_size, available_size.height as i32 / -2);
                self.show(write, terminal_size)?;
                Ok(true)
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
                self.scroll = 0;
                if let Some(ref mut cursor) = self.cursor {
                    *cursor = 0;
                }
                self.show(write, terminal_size)?;
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char('e'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::End, ..
            } => {
                let content_height = self.content_height(available_size);
                self.scroll = 0
                    .max(content_height as i32 - available_size.height as i32)
                    as usize;

                if let Some(ref mut cursor) = self.cursor {
                    *cursor = content_height - 1;
                }
                self.show(write, terminal_size)?;
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char(' '),
                ..
            } => {
                self.is_filtering = !self.is_filtering;
                self.on_filter_changed(write, terminal_size)?;
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::CONTROL,
            }
            | KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                if self.filter.len() > 0 {
                    self.filter.remove(self.filter.len() - 1);
                }
                self.on_filter_changed(write, terminal_size)?;
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Char('w'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                self.filter.clear();
                self.on_filter_changed(write, terminal_size)?;
                Ok(true)
            }
            KeyEvent {
                code: KeyCode::Esc, ..
            }
            | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            } => {
                if self.is_filtering {
                    self.is_filtering = false;
                    self.filter.clear();
                    self.on_filter_changed(write, terminal_size)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            key_event => {
                if let (true, Some(c)) =
                    (self.is_filtering, input::key_to_char(key_event))
                {
                    self.filter.push(c);
                    self.on_filter_changed(write, terminal_size)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }

    fn filtered_lines(&self) -> impl Iterator<Item = &str> {
        self.content
            .lines()
            .filter(move |line| fuzzy_matches(line, &self.filter[..]))
    }

    fn content_height(&self, available_size: AvailableSize) -> usize {
        if self.cursor.is_some() {
            self.filtered_lines().count()
        } else {
            let width = available_size.width;
            self.filtered_lines()
                .map(|l| (l.len() + width - 1) / width)
                .sum()
        }
    }

    fn scroll(&mut self, available_size: AvailableSize, delta: i32) {
        let content_height = self.content_height(available_size);
        if let Some(ref mut cursor) = self.cursor {
            move_cursor(
                &mut self.scroll,
                cursor,
                available_size,
                content_height,
                delta,
            );
        } else {
            self.scroll = (self.scroll as i32 + delta)
                .min(content_height as i32 - available_size.height as i32)
                .max(0) as usize;
        }
    }

    fn on_filter_changed<W>(
        &mut self,
        writer: &mut W,
        terminal_size: TerminalSize,
    ) -> Result<()>
    where
        W: Write,
    {
        self.scroll = 0;
        self.cursor = self.cursor.map(|_| 0);
        self.show(writer, terminal_size)
    }
}
