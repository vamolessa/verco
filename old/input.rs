use std::time::Duration;

use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};
use rustyline::{error::ReadlineError, Editor};

use crate::tui_util::TerminalSize;

pub enum Event {
    None,
    Resize(TerminalSize),
    Key(KeyEvent),
}

pub fn poll_event() -> Event {
    if event::poll(Duration::from_millis(10)).unwrap() {
        match event::read().unwrap() {
            event::Event::Resize(width, height) => {
                Event::Resize(TerminalSize { width, height })
            }
            event::Event::Key(key) => Event::Key(key),
            _ => Event::None,
        }
    } else {
        Event::None
    }
}

pub fn key_to_char(key: KeyEvent) -> Option<char> {
    match key {
        KeyEvent {
            code: KeyCode::Char(c),
            modifiers: m,
        } => {
            if m == KeyModifiers::SHIFT {
                Some(c.to_ascii_uppercase())
            } else if m.is_empty() {
                Some(c)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn read_line(initial: &str) -> Result<String, ReadlineError> {
    let mut readline = Editor::<()>::new();
    match readline.readline_with_initial("", (initial, "")) {
        Ok(line) => Ok(line),
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
            Ok("".into())
        }
        Err(error) => Err(error),
    }
}
