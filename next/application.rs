use std::{
    io,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use crossterm::{event, terminal};

use crate::{
    backend::Backend,
    mode::{self, ModeContext, ModeKind, ModeResponse, ModeStatus},
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
    ModeChange(ModeKind),
}

#[derive(Clone)]
pub struct EventSender(mpsc::SyncSender<Event>);
impl EventSender {
    pub fn send_response(&self, result: ModeResponse) {
        let _ = self.0.send(Event::Response(result));
    }
    
    pub fn send_mode_change(&self, mode: ModeKind) {
        let _ = self.0.send(Event::ModeChange(mode));
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

#[derive(Default)]
struct Application {
    current_mode: ModeKind,

    status_mode: mode::status::Mode,
    log_mode: mode::log::Mode,

    spinner_state: u8,
}
impl Application {
    pub fn enter_mode(&mut self, ctx: &ModeContext, mode: ModeKind) {
        self.current_mode = mode;
        match &self.current_mode {
            ModeKind::Status => self.status_mode.on_enter(ctx),
            ModeKind::Log => self.log_mode.on_enter(ctx),
            ModeKind::RevisionDetails(revision) => todo!(),
        }
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> bool {
        let status = match &self.current_mode {
            ModeKind::Status => self.status_mode.on_key(ctx, key),
            ModeKind::Log => self.log_mode.on_key(ctx, key),
            ModeKind::RevisionDetails(revision) => todo!(),
        };

        if !status.pending_input {
            if key.is_cancel() {
                return false;
            }

            match key {
                Key::Char('s') => self.enter_mode(ctx, ModeKind::Status),
                Key::Char('l') => self.enter_mode(ctx, ModeKind::Log),
                _ => (),
            }
        }

        true
    }

    pub fn on_response(&mut self, response: ModeResponse) {
        match response {
            ModeResponse::Status(response) => {
                self.status_mode.on_response(response)
            }
            ModeResponse::Log(response) => self.log_mode.on_response(response),
            ModeResponse::RevisionDetails(response) => todo!(),
        }
    }

    pub fn draw_header(&mut self, drawer: &mut Drawer) {
        self.spinner_state = self.spinner_state.wrapping_add(1);

        let header_info = match &self.current_mode {
            ModeKind::Status => self.status_mode.header(),
            ModeKind::Log => self.log_mode.header(),
            ModeKind::RevisionDetails(_) => todo!(),
        };
        drawer.header(header_info, self.spinner_state);
    }

    pub fn draw_body(&self, drawer: &mut Drawer) {
        match &self.current_mode {
            ModeKind::Status => self.status_mode.draw(drawer),
            ModeKind::Log => self.log_mode.draw(drawer),
            ModeKind::RevisionDetails(_) => todo!(),
        }
        drawer.clear_to_bottom();
    }
}

pub fn run(backend: Arc<dyn Backend>) {
    let viewport_size = match terminal::size() {
        Ok((width, height)) => (width, height),
        Err(_) => return,
    };

    let (event_sender, event_receiver) = mpsc::sync_channel(0);

    let mut ctx = ModeContext {
        backend,
        event_sender: EventSender(event_sender.clone()),
        viewport_size,
    };

    let mut application = Application::default();
    application.enter_mode(&ctx, ModeKind::default());

    thread::spawn(|| console_events_loop(event_sender));

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    loop {
        let mut draw_body = true;

        let timeout = Duration::from_millis(100);
        match event_receiver.recv_timeout(timeout) {
            Ok(Event::Key(key)) => {
                if !application.on_key(&ctx, key) {
                    break;
                }
            }
            Ok(Event::Resize(width, height)) => {
                ctx.viewport_size = (width, height);
            }
            Ok(Event::Response(response)) => application.on_response(response),
            Ok(Event::ModeChange(mode)) => application.enter_mode(&ctx, mode),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                draw_body = false;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        let mut drawer = Drawer::new(&mut stdout, viewport_size);
        application.draw_header(&mut drawer);
        if draw_body {
            application.draw_body(&mut drawer);
        }

        use io::Write;
        stdout.flush().unwrap();
    }
}

