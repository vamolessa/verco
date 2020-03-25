use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::ctrlc_handler::CtrlcHandler;
use std::time::Duration;

pub fn read_key(ctrlc_handler: &mut CtrlcHandler) -> crossterm::Result<KeyEvent> {
    loop {
        if ctrlc_handler.get() {
            return Ok(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            });
        }

        if event::poll(Duration::from_millis(10))? {
            match event::read()? {
                Event::Key(key_event) => {
                    return Ok(key_event);
                }
                _ => (),
            }
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

pub fn read_line() -> crossterm::Result<String> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let len = line.trim_end_matches(&['\r', '\n'][..]).len();
    line.truncate(len);
    Ok(line)
}
