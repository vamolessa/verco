use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::{Print, ResetColor, SetBackgroundColor},
    terminal::{self, Clear, ClearType},
    QueueableCommand, Result,
};

use std::io::Write;

use crate::select::{move_cursor, SELECTED_BG_COLOR};

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

    pub fn show<W>(&self, write: &mut W) -> Result<()>
    where
        W: Write,
    {
        let available_size = Self::available_size();
        write.queue(cursor::MoveTo(0, 1))?;
        for (i, line) in self
            .content
            .lines()
            .skip(self.scroll)
            .take(available_size.1 - 1)
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
    ) -> Result<bool>
    where
        W: Write,
    {
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
                self.scroll(1);
                self.show(write)?;
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
                self.scroll(-1);
                self.show(write)?;
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
                self.scroll(Self::available_size().1 as i32 / 2);
                self.show(write)?;
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
                self.scroll(Self::available_size().1 as i32 / -2);
                self.show(write)?;
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
                self.show(write)?;
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
                    self.content_height() as i32
                        - Self::available_size().1 as i32,
                ) as usize;
                self.show(write)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn available_size() -> (usize, usize) {
        let terminal_size = terminal::size().unwrap_or((0, 0));
        (terminal_size.0 as usize, terminal_size.1 as usize - 2)
    }

    fn content_height(&self) -> usize {
        let width = Self::available_size().0;
        self.content
            .lines()
            .map(|l| (l.len() + width - 1) / width)
            .sum()
    }

    fn scroll(&mut self, delta: i32) {
        if let Some(ref mut cursor) = self.cursor {
            let line_count = self.content.lines().count();
            move_cursor(&mut self.scroll, cursor, line_count, delta);
        } else {
            self.scroll = (self.scroll as i32 + delta)
                .min(
                    self.content_height() as i32
                        - Self::available_size().1 as i32,
                )
                .max(0) as usize;
        }
    }
}
