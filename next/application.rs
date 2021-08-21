use std::{
    io,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use crossterm::{event, terminal};

use crate::{
    backend::Backend,
    mode::{self, ModeContext, ModeKind, ModeResponse},
    ui::Drawer,
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

    pub fn is_submit(&self) -> bool {
        matches!(self, Self::Enter | Self::Char('\n') | Self::Ctrl('m'))
    }

    pub fn is_cancel(&self) -> bool {
        matches!(self, Self::Esc | Self::Ctrl('c'))
    }
}

enum Event {
    Key(Key),
    Resize(u16, u16),
    Response(ModeResponse),
}

#[derive(Clone)]
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

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut spinner_state = 0u8;

    loop {
        let mut draw_body = true;

        let timeout = Duration::from_millis(100);
        match event_receiver.recv_timeout(timeout) {
            Ok(Event::Key(key)) => {
                let input_status = match current_mode {
                    ModeKind::Status => status_mode.on_key(&mode_ctx, key),
                };

                if !input_status.pending {
                    if key.is_cancel() {
                        break;
                    }

                    match key {
                        Key::Char('s') => {
                            current_mode = ModeKind::Status;
                            status_mode.on_enter(&mode_ctx);
                        }
                        _ => (),
                    }
                }
            }
            Ok(Event::Resize(width, height)) => {
                mode_ctx.viewport_size = (width, height);
            }
            Ok(Event::Response(ModeResponse::Status(response))) => {
                status_mode.on_response(response);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                spinner_state = spinner_state.wrapping_add(1);
                draw_body = false;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        let mut drawer = Drawer::new(&mut stdout, viewport_size);

        let mut header_info = match current_mode {
            ModeKind::Status => status_mode.header(),
        };
        drawer.header(header_info, spinner_state);

        if draw_body {
            match current_mode {
                ModeKind::Status => status_mode.draw(&mut drawer),
            }
            drawer.clear_to_bottom();
        } else {
            use io::Write;
            stdout.flush().unwrap();
        }
    }
}

