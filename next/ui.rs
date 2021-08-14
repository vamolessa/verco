use std::io::{StdoutLock, Write};

use crossterm::{self, cursor, style, terminal};

use crate::mode::{Output, ReadLine, SelectMenu};

pub trait Draw {
    fn draw(&self, drawer: &mut Drawer);
}

pub struct Drawer<'a> {
    stdout: StdoutLock<'a>,
    viewport_size: (u16, u16),
}

impl<'a> Drawer<'a> {
    pub fn new(stdout: StdoutLock<'a>, viewport_size: (u16, u16)) -> Self {
        Self {
            stdout,
            viewport_size,
        }
    }

    pub fn header(&mut self, mode_name: &str) {
        let background_color = style::Color::DarkYellow;
        let foreground_color = style::Color::Black;

        crossterm::queue!(
            &mut self.stdout,
            cursor::MoveTo(0, 0),
            style::SetBackgroundColor(background_color),
            style::SetForegroundColor(foreground_color),
            style::Print(' '),
            style::SetBackgroundColor(foreground_color),
            style::SetForegroundColor(background_color),
            style::Print(' '),
            style::Print(mode_name),
            style::Print(' '),
            style::SetBackgroundColor(background_color),
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1),
            style::ResetColor,
        )
        .unwrap();
    }

    pub fn text(&mut self, text: &str) {
        self.stdout.write_all(text.as_bytes()).unwrap();
    }

    pub fn toggle(&mut self, on: bool) {
        let state_text = if on { "+ " } else { "  " };
        self.stdout.write_all(state_text.as_bytes()).unwrap();
    }

    pub fn output(&mut self, output: &Output) {
        for line in output.lines_from_scroll() {
            crossterm::queue!(
                &mut self.stdout,
                style::Print(line),
                terminal::Clear(terminal::ClearType::UntilNewLine)
            )
            .unwrap();
        }
    }

    pub fn readline(&mut self, readline: &ReadLine) {
        crossterm::queue!(
            &mut self.stdout,
            style::SetBackgroundColor(style::Color::Black),
            style::SetForegroundColor(style::Color::White),
            style::Print(readline.input()),
            style::SetBackgroundColor(style::Color::DarkRed),
            style::Print(' '),
            style::SetBackgroundColor(style::Color::Black),
        )
        .unwrap();
    }

    pub fn select_menu<'entries, I, E>(
        &mut self,
        select: &SelectMenu,
        entries: I,
    ) where
        I: 'entries + Iterator<Item = &'entries E>,
        E: 'entries + Draw,
    {
        let cursor_index = select.cursor();

        crossterm::queue!(
            &mut self.stdout,
            style::SetBackgroundColor(style::Color::Black),
            style::SetForegroundColor(style::Color::White),
        )
        .unwrap();

        for (i, entry) in entries
            .enumerate()
            .skip(select.scroll())
            .take(self.viewport_size.1 as _)
        {
            if i == cursor_index {
                crossterm::queue!(
                    &mut self.stdout,
                    style::SetBackgroundColor(style::Color::DarkGrey),
                )
                .unwrap();
            }

            entry.draw(self);

            crossterm::queue!(
                &mut self.stdout,
                terminal::Clear(terminal::ClearType::UntilNewLine),
                cursor::MoveToNextLine(1),
            )
            .unwrap();

            if i == cursor_index {
                crossterm::queue!(
                    &mut self.stdout,
                    style::SetBackgroundColor(style::Color::Black),
                )
                .unwrap();
            }
        }
    }
}

impl<'a> Drop for Drawer<'a> {
    fn drop(&mut self) {
        crossterm::execute!(
            &mut self.stdout,
            style::SetBackgroundColor(style::Color::Black),
            terminal::Clear(terminal::ClearType::FromCursorDown),
        )
        .unwrap();
    }
}

