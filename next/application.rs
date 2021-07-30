use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
};

use crossterm::{event, terminal};

use crate::{
    backend::Backend,
    mode::{ModeManager, ModeResponse},
    ui,
};

static VIEWPORT_WIDTH: AtomicUsize = AtomicUsize::new(0);
static VIEWPORT_HEIGHT: AtomicUsize = AtomicUsize::new(0);

pub fn viewport_size() -> (u16, u16) {
    let width = VIEWPORT_WIDTH.load(Ordering::Relaxed);
    let height = VIEWPORT_HEIGHT.load(Ordering::Relaxed);
    (width as _, height as _)
}

fn resize_viewport(width: u16, height: u16) {
    VIEWPORT_WIDTH.store(width as _, Ordering::Relaxed);
    VIEWPORT_HEIGHT.store(height as _, Ordering::Relaxed);
}

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

pub enum InputEvent {
    Key(Key),
    Redraw,
}
impl InputEvent {
    pub fn next() -> Self {
        loop {
            match event::read().unwrap() {
                event::Event::Key(key) => {
                    return Self::Key(Key::from_key_event(key));
                }
                event::Event::Mouse(_) => (),
                event::Event::Resize(width, height) => {
                    resize_viewport(width, height);
                    return Self::Redraw;
                }
            }
        }
    }
}

pub enum ApplicationEvent {
    Input(InputEvent),
    Response(Result<ModeResponse, String>),
}

#[derive(Clone)]
pub struct ModeResponseSender(mpsc::SyncSender<ApplicationEvent>);
impl ModeResponseSender {
    pub fn send(&self, result: Result<ModeResponse, String>) {
        let _ = self.0.send(ApplicationEvent::Response(result));
    }
}

pub struct Application {
    backend: Arc<dyn Backend>,
    modes: ModeManager,
}

pub fn run(backend: Arc<dyn Backend>) {
    match terminal::size() {
        Ok((width, height)) => resize_viewport(width, height),
        Err(_) => return,
    };

    let (event_sender, event_receiver) = mpsc::sync_channel(0);

    let mut app = Application {
        backend,
        modes: ModeManager::new(ModeResponseSender(event_sender.clone())),
    };

    loop {
        match InputEvent::next() {
            InputEvent::Key(Key::Esc | Key::Ctrl('c') | Key::Char('q')) => {
                break
            }
            InputEvent::Key(key) => app.modes.on_key(app.backend.clone(), key),
            InputEvent::Redraw => (),
        }

        app.modes.draw();
    }
}

