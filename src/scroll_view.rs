use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::{Print, ResetColor, SetBackgroundColor},
    terminal::{Clear, ClearType},
    QueueableCommand, Result,
};

use std::io::Write;

use crate::tui_util::{
    move_cursor, AvailableSize, TerminalSize, SELECTED_BG_COLOR,
};

#[derive(Default)]
pub struct ScrollView {
    content: String,
    scroll: usize,
    cursor: Option<usize>,
}

impl ScrollView {
    pub fn set_content(&mut self, content: &str, has_cursor: bool) {
        self.scroll = 0;
        self.cursor = if has_cursor { Some(0) } else { None };
        self.content.clear();
        self.content.push_str(content);
    }

    pub fn show<W>(
        &self,
        write: &mut W,
        terminal_size: TerminalSize,
    ) -> Result<()>
    where
        W: Write,
    {
        let available_size = AvailableSize::from_temrinal_size(terminal_size);
        write.queue(cursor::MoveTo(0, 1))?;
        for (i, line) in self
            .content
            .lines()
            .skip(self.scroll)
            .take(available_size.height - 1)
            .enumerate()
        {
            if Some(i) == self.cursor {
                write.queue(SetBackgroundColor(SELECTED_BG_COLOR))?;
            }

            queue!(
                write,
                Clear(ClearType::CurrentLine),
                Print(line),
                cursor::MoveToNextLine(1),
            )?;

            if Some(i) == self.cursor {
                write.queue(ResetColor)?;
            }
        }
        write.queue(Clear(ClearType::FromCursorDown))?;

        Ok(())
    }

    pub fn update<W>(
        &mut self,
        write: &mut W,
        key_event: &KeyEvent,
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
            }
            | KeyEvent {
                code: KeyCode::Char(' '),
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
                self.scroll = 0.max(
                    self.content_height(available_size) as i32
                        - available_size.height as i32,
                ) as usize;
                self.show(write, terminal_size)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn content_height(&self, available_size: AvailableSize) -> usize {
        let width = available_size.width;
        self.content
            .lines()
            .map(|l| (l.len() + width - 1) / width)
            .sum()
    }

    fn scroll(&mut self, available_size: AvailableSize, delta: i32) {
        if let Some(ref mut cursor) = self.cursor {
            let line_count = self.content.lines().count();
            move_cursor(
                &mut self.scroll,
                cursor,
                available_size,
                line_count,
                delta,
            );
        } else {
            self.scroll = (self.scroll as i32 + delta)
                .min(
                    self.content_height(available_size) as i32
                        - available_size.height as i32,
                )
                .max(0) as usize;
        }
    }
}
