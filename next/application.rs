use std::{
    sync::{mpsc, Arc},
    thread,
};

use crossterm::{event, terminal};

use crate::{
    backend::Backend,
    mode::{self, Mode, ModeContext, ModeKind, ModeResponse},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Key {
    None,
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    Char(char),
    Ctrl(char),
    Esc,
}
impl Key {
    pub fn from_key_event(ev: event::KeyEvent) -> Self {
        match ev.code {
            event::KeyCode::Backspace => Self::Backspace,
            event::KeyCode::Enter => Self::Enter,
            event::KeyCode::Left => Self::Left,
            event::KeyCode::Right => Self::Right,
            event::KeyCode::Up => Self::Up,
            event::KeyCode::Down => Self::Down,
            event::KeyCode::Home => Self::Home,
            event::KeyCode::End => Self::End,
            event::KeyCode::PageUp => Self::PageUp,
            event::KeyCode::PageDown => Self::PageDown,
            event::KeyCode::Tab => Self::Tab,
            event::KeyCode::BackTab => Self::BackTab,
            event::KeyCode::Delete => Self::Delete,
            event::KeyCode::Insert => Self::Insert,
            event::KeyCode::F(_) => Self::None,
            event::KeyCode::Char(mut c) => {
                if ev.modifiers & event::KeyModifiers::ALT
                    != event::KeyModifiers::NONE
                {
                    return Self::None;
                }

                if ev.modifiers & event::KeyModifiers::SHIFT
                    != event::KeyModifiers::NONE
                {
                    c = c.to_ascii_uppercase();
                }
                if ev.modifiers & event::KeyModifiers::CONTROL
                    != event::KeyModifiers::NONE
                {
                    Self::Ctrl(c)
                } else {
                    Self::Char(c)
                }
            }
            event::KeyCode::Null => Self::None,
            event::KeyCode::Esc => Self::Esc,
        }
    }
}

enum Event {
    Key(Key),
    Resize(u16, u16),
    Response(ModeResponse),
}

pub struct ModeResponseSender(mpsc::SyncSender<Event>);
impl ModeResponseSender {
    pub fn send(&self, result: ModeResponse) {
        let _ = self.0.send(Event::Response(result));
    }
}

fn console_events_loop(sender: mpsc::SyncSender<Event>) {
    loop {
        let event = match event::read() {
            Ok(event) => event,
            Err(_) => break,
        };
        match event {
            event::Event::Key(key) => {
                let event = Event::Key(Key::from_key_event(key));
                if sender.send(event).is_err() {
                    break;
                }
            }
            event::Event::Mouse(_) => (),
            event::Event::Resize(width, height) => {
                let event = Event::Resize(width, height);
                if sender.send(event).is_err() {
                    break;
                }
            }
        }
    }
}

pub fn run(backend: Arc<dyn Backend>) {
    let viewport_size = match terminal::size() {
        Ok((width, height)) => (width, height),
        Err(_) => return,
    };

    let (event_sender, event_receiver) = mpsc::sync_channel(0);

    let mut mode_ctx = ModeContext {
        backend,
        response_sender: ModeResponseSender(event_sender.clone()),
        viewport_size,
    };

    let mut current_mode = ModeKind::Status;
    let mut status_mode = mode::status::Mode::default();

    status_mode.on_enter(&mode_ctx);

    thread::spawn(|| console_events_loop(event_sender));

    loop {
        let event = match event_receiver.recv() {
            Ok(event) => event,
            Err(_) => break,
        };
        match event {
            Event::Key(Key::Esc | Key::Ctrl('c') | Key::Char('q')) => break,
            Event::Key(Key::Char('s')) => {
                current_mode = ModeKind::Status;
                status_mode.on_enter(&mode_ctx);
            }
            Event::Key(key) => match current_mode {
                ModeKind::Status => status_mode.on_key(&mode_ctx, key),
            },
            Event::Resize(width, height) => {
                mode_ctx.viewport_size = (width, height);
            }
            Event::Response(response) => {
                status_mode.on_response(&response);
            }
        }

        match current_mode {
            ModeKind::Status => status_mode.draw(mode_ctx.viewport_size),
        }
    }
}

