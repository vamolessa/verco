use std::{
    io,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use crate::{
    backend::Backend,
    mode::{self, ModeContext, ModeKind, ModeResponse},
    platform::{Key, Platform, PlatformEventReader},
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

    pub fn is_waiting_response(&self) -> bool {
        match &self.current_mode {
            ModeKind::Status => self.status_mode.is_waiting_response(),
            ModeKind::Log => self.log_mode.is_waiting_response(),
            ModeKind::RevisionDetails(_) => {
                self.revision_details_mode.is_waiting_response()
            }
            ModeKind::Branches => self.branches_mode.is_waiting_response(),
            ModeKind::Tags => self.tags_mode.is_waiting_response(),
        }
    }

    pub fn draw_header(&mut self, drawer: &mut Drawer) {
        let spinner = [b'-', b'\\', b'|', b'/'];
        self.spinner_state = (self.spinner_state + 1) % spinner.len() as u8;
        let spinner = match self.is_waiting_response() {
            true => spinner[self.spinner_state as usize],
            false => b' ',
        };

        let header = match &self.current_mode {
            ModeKind::Status => self.status_mode.header(),
            ModeKind::Log => self.log_mode.header(),
            ModeKind::RevisionDetails(_) => self.revision_details_mode.header(),
            ModeKind::Branches => self.branches_mode.header(),
            ModeKind::Tags => self.tags_mode.header(),
        };
        drawer.header(header, spinner);
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

fn terminal_event_loop(
    mut event_reader: PlatformEventReader,
    sender: mpsc::SyncSender<Event>,
) {
    let mut keys = Vec::new();
    loop {
        keys.clear();
        let mut resize = None;

        event_reader.read_terminal_events(&mut keys, &mut resize);

        for &key in &keys {
            if sender.send(Event::Key(key)).is_err() {
                break;
            }
        }
        if let Some(resize) = resize {
            if sender.send(Event::Resize(resize.0, resize.1)).is_err() {
                break;
            }
        }
    }
}

pub fn run(platform_event_reader: PlatformEventReader, backend: Arc<dyn Backend>) {
    let (event_sender, event_receiver) = mpsc::sync_channel(1);

    let mut ctx = ModeContext {
        backend,
        event_sender: EventSender(event_sender.clone()),
        viewport_size: Platform::terminal_size(),
    };

    let _ = thread::spawn(move || {
        terminal_event_loop(platform_event_reader, event_sender);
    });

    let mut application = Application::default();
    application.enter_mode(&ctx, ModeKind::default());

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut stdout_buf = Vec::new();

    let mut counter = 0;

    const TIMEOUT: Duration = Duration::from_millis(100);

    loop {
        let mut draw_body = true;

        let is_waiting_response = application.is_waiting_response();

        let event = if is_waiting_response {
            event_receiver.recv_timeout(TIMEOUT)
        } else {
            event_receiver
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        };

        let event_disc = std::mem::discriminant(&event);
        let mut is_resize = false;
        match event {
            Ok(Event::Key(key)) => {
                if !application.on_key(&ctx, key) {
                    break;
                }
            }
            Ok(Event::Resize(width, height)) => {
                ctx.viewport_size = (width, height);
                is_resize = true;
            }
            Ok(Event::Response(response)) => application.on_response(response),
            Ok(Event::ModeChange(mode)) => application.enter_mode(&ctx, mode),
            Ok(Event::ModeRefresh(mode)) => {
                application.refresh_mode(&ctx, mode)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                draw_body = false;
                counter += 1;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }

        let mut drawer = Drawer::new(stdout_buf, ctx.viewport_size);
        application.draw_header(&mut drawer);
        drawer.fmt(format_args!(
            "{:?} {} is_resize: {} ",
            event_disc, counter, is_resize
        ));
        drawer.next_line();

        if draw_body {
            application.draw_body(&mut drawer);
        }
        stdout_buf = drawer.take_buf();

        use io::Write;
        stdout.write_all(&stdout_buf).unwrap();
        stdout.flush().unwrap();
    }
}

