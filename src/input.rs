use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use rustyline::{error::ReadlineError, Editor};

pub fn read_key() -> crossterm::Result<KeyEvent> {
    loop {
        match event::read()? {
            Event::Key(key_event) => {
                return Ok(key_event);
            }
            _ => (),
        }
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

pub fn read_line() -> Result<String, ReadlineError> {
    let mut readline = Editor::<()>::new();
    match readline.readline("") {
        Ok(line) => Ok(line),
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => Ok("".into()),
        Err(error) => Err(error),
    }
}
