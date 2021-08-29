use std::{
    io,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use crate::{
    backend::Backend,
    mode::{self, ModeContext, ModeKind, ModeResponse},
    platform::{Key, PlatformEvent, Platform},
    ui::Drawer,
};

enum Event {
    Key(Key),
    Resize(u16, u16),
    Response(ModeResponse),
    ModeChange(ModeKind),
    ModeRefresh(ModeKind),
}

#[derive(Clone)]
pub struct EventSender(mpsc::SyncSender<Event>);
impl EventSender {
    pub fn send_response(&self, result: ModeResponse) {
        self.0.send(Event::Response(result)).unwrap();
    }

    pub fn send_mode_change(&self, mode: ModeKind) {
        self.0.send(Event::ModeChange(mode)).unwrap();
    }

    pub fn send_mode_refresh(&self, mode: ModeKind) {
        self.0.send(Event::ModeRefresh(mode)).unwrap();
    }
}

fn console_events_loop(sender: mpsc::SyncSender<Event>) {
    loop {
        let event = Platform::next_terminal_event();
        match event {
            PlatformEvent::Key(key) => {
                let event = Event::Key(key);
                if sender.send(event).is_err() {
                    break;
                }
            }
            PlatformEvent::Resize(width, height) => {
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
    revision_details_mode: mode::revision_details::Mode,
    branches_mode: mode::branches::Mode,
    tags_mode: mode::tags::Mode,

    spinner_state: u8,
}
impl Application {
    pub fn enter_mode(&mut self, ctx: &ModeContext, mode: ModeKind) {
        self.current_mode = mode;
        match &self.current_mode {
            ModeKind::Status => self.status_mode.on_enter(ctx),
            ModeKind::Log => self.log_mode.on_enter(ctx),
            ModeKind::RevisionDetails(revision) => {
                self.revision_details_mode.on_enter(ctx, revision);
            }
            ModeKind::Branches => self.branches_mode.on_enter(ctx),
            ModeKind::Tags => self.tags_mode.on_enter(ctx),
        }
    }

    pub fn refresh_mode(&mut self, ctx: &ModeContext, mode: ModeKind) {
        if std::mem::discriminant(&self.current_mode)
            == std::mem::discriminant(&mode)
        {
            self.enter_mode(ctx, mode);
        }
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> bool {
        let status = match &self.current_mode {
            ModeKind::Status => self.status_mode.on_key(ctx, key),
            ModeKind::Log => self.log_mode.on_key(ctx, key),
            ModeKind::RevisionDetails(revision) => {
                self.revision_details_mode.on_key(ctx, revision, key)
            }
            ModeKind::Branches => self.branches_mode.on_key(ctx, key),
            ModeKind::Tags => self.tags_mode.on_key(ctx, key),
        };

        if !status.pending_input {
            if key.is_cancel() {
                return false;
            }

            match key {
                Key::Char('s') => self.enter_mode(ctx, ModeKind::Status),
                Key::Char('l') => self.enter_mode(ctx, ModeKind::Log),
                Key::Char('b') => self.enter_mode(ctx, ModeKind::Branches),
                Key::Char('t') => self.enter_mode(ctx, ModeKind::Tags),
                _ => (),
            }
        }

        true
    }

    pub fn on_response(&mut self, response: ModeResponse) {
        match response {
            ModeResponse::Status(response) => {
                self.status_mode.on_response(response);
            }
            ModeResponse::Log(response) => self.log_mode.on_response(response),
            ModeResponse::RevisionDetails(response) => {
                self.revision_details_mode.on_response(response);
            }
            ModeResponse::Branches(response) => {
                self.branches_mode.on_response(response);
            }
            ModeResponse::Tags(response) => {
                self.tags_mode.on_response(response);
            }
        }
    }

    pub fn draw_header(&mut self, drawer: &mut Drawer) {
        self.spinner_state = self.spinner_state.wrapping_add(1);

        let header_info = match &self.current_mode {
            ModeKind::Status => self.status_mode.header(),
            ModeKind::Log => self.log_mode.header(),
            ModeKind::RevisionDetails(_) => self.revision_details_mode.header(),
            ModeKind::Branches => self.branches_mode.header(),
            ModeKind::Tags => self.tags_mode.header(),
        };
        drawer.header(header_info, self.spinner_state);
    }

    pub fn draw_body(&self, drawer: &mut Drawer) {
        match &self.current_mode {
            ModeKind::Status => self.status_mode.draw(drawer),
            ModeKind::Log => self.log_mode.draw(drawer),
            ModeKind::RevisionDetails(_) => {
                self.revision_details_mode.draw(drawer);
            }
            ModeKind::Branches => self.branches_mode.draw(drawer),
            ModeKind::Tags => self.tags_mode.draw(drawer),
        }
        drawer.clear_to_bottom();
    }
}

pub fn run(backend: Arc<dyn Backend>) {
    let viewport_size = Platform::terminal_size();

    let (event_sender, event_receiver) = mpsc::sync_channel(1);

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
    let mut stdout_buf = Vec::new();

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
            Ok(Event::ModeRefresh(mode)) => {
                application.refresh_mode(&ctx, mode)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => draw_body = false,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        let mut drawer = Drawer::new(stdout_buf, viewport_size);
        application.draw_header(&mut drawer);
        if draw_body {
            application.draw_body(&mut drawer);
        }
        stdout_buf = drawer.take_buf();

        use io::Write;
        stdout.write_all(&stdout_buf).unwrap();
        stdout.flush().unwrap();
    }
}

