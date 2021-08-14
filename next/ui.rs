use std::io::{StdoutLock, Write};

use crossterm::{self, cursor, style, terminal};

use crate::mode::SelectMenu;

pub enum TextKind {
    Normal,
    Label,
}
impl TextKind {
    pub fn to_color(self) -> style::Color {
        match self {
            Self::Normal => style::Color::White,
            Self::Label => style::Color::DarkYellow,
        }
    }
}

pub trait Draw {
    fn draw(&self, drawer: &mut Drawer);
}

pub struct Drawer<'a> {
    stdout: StdoutLock<'a>,
}

impl<'a> Drawer<'a> {
    pub fn new(stdout: StdoutLock<'a>) -> Self {
        Self { stdout }
    }

    pub fn header(&mut self, mode_name: &str) {
        let dark_background_color = style::Color::DarkGreen;
        let light_background_color = style::Color::Green;
        let foreground_color = style::Color::White;

        crossterm::queue!(
            &mut self.stdout,
            cursor::MoveTo(0, 0),
            style::SetBackgroundColor(light_background_color),
            style::SetForegroundColor(foreground_color),
            style::Print(' '),
            style::SetBackgroundColor(dark_background_color),
            style::Print(mode_name),
            style::SetBackgroundColor(light_background_color),
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1),
            style::ResetColor,
        )
        .unwrap();
    }

    pub fn text(&mut self, text: &str, kind: TextKind) {
        crossterm::queue!(
            &mut self.stdout,
            style::SetForegroundColor(kind.to_color()),
            style::Print(text)
        )
        .unwrap();
    }

    pub fn toggle(&mut self, on: bool) {
        let state_text = if on { "+ " } else { "  " };
        self.stdout.write_all(state_text.as_bytes()).unwrap();
    }

    pub fn next_line(&mut self) {
        crossterm::queue!(
            &mut self.stdout,
            terminal::Clear(terminal::ClearType::UntilNewLine),
            cursor::MoveToNextLine(1)
        )
        .unwrap();
    }

    pub fn output(&mut self, output: &str) {
        write!(&mut self.stdout, "output:\n{}\n----\n", output).unwrap();
    }

    pub fn select_menu<'entries, I, E>(
        &mut self,
        select: &SelectMenu,
        entries: I,
        viewport_size: (u16, u16),
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
            .take(viewport_size.1 as _)
        {
            if i == cursor_index {
                crossterm::queue!(
                    &mut self.stdout,
                    style::SetBackgroundColor(style::Color::Grey),
                )
                .unwrap();
            }

            entry.draw(self);

            if i == cursor_index {
                crossterm::queue!(
                    &mut self.stdout,
                    style::SetBackgroundColor(style::Color::Black),
                )
                .unwrap();
            }

            crossterm::queue!(
                &mut self.stdout,
                terminal::Clear(terminal::ClearType::UntilNewLine),
            )
            .unwrap();
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

