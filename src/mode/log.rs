use std::thread;

use crate::{
    application::Key,
    backend::{Backend, BackendResult, LogEntry},
    mode::{
        HeaderInfo, ModeContext, ModeKind, ModeResponse, ModeStatus, Output,
        SelectMenu,
    },
    ui::{Color, Drawer, SelectEntryDraw},
};

pub enum Response {
    Refresh(BackendResult<Vec<LogEntry>>),
}

enum WaitOperation {
    Refresh,
    Checkout,
    Merge,
    Fetch,
    Pull,
    Push,
}

enum State {
    Idle,
    Waiting(WaitOperation),
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl SelectEntryDraw for LogEntry {
    fn draw(&self, drawer: &mut Drawer, hovered: bool, full: bool) -> usize {
        fn color(color: Color, hovered: bool) -> Color {
            if hovered {
                Color::White
            } else {
                color
            }
        }

        let author = match self.author.char_indices().nth(12) {
            Some((i, _)) => &self.author[..i],
            None => &self.author,
        };

        let mut total_chars = self.graph.chars().count()
            + 1
            + self.hash.chars().count()
            + 1
            + self.date.chars().count()
            + 1
            + author.chars().count()
            + 1;

        if !self.refs.is_empty() {
            total_chars += self.refs.chars().count() + 3;
        }

        let available_width =
            (drawer.viewport_size.0 as usize).saturating_sub(total_chars);
        let message = match self.message.char_indices().nth(available_width) {
            Some((i, _)) => &self.message[..i],
            None => &self.message,
        };

        let (refs_begin, refs_end) = match &self.refs[..] {
            "" => ("", ""),
            _ => ("(", ") "),
        };

        drawer.write(&format_args!(
            "{}{} {}{} {}{} {}{} {}{}{}{}{}{}",
            color(Color::White, hovered),
            &self.graph,
            color(Color::Yellow, hovered),
            &self.hash,
            color(Color::Blue, hovered),
            &self.date,
            color(Color::Green, hovered),
            author,
            color(Color::Red, hovered),
            refs_begin,
            &self.refs,
            refs_end,
            color(Color::White, hovered),
            message,
        ));

        1
    }
}

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<LogEntry>,
    output: Output,
    select: SelectMenu,
}
impl Mode {
    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::Waiting(_) = self.state {
            return;
        }
        self.state = State::Waiting(WaitOperation::Refresh);

        self.output.set(String::new());
        request(ctx, |_| Ok(()));
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> ModeStatus {
        let available_height = ctx.viewport_size.1.saturating_sub(1) as usize;
        self.select
            .on_key(self.entries.len(), available_height, key);

        if let Key::Char('d') = key {
            let index = self.select.cursor();
            if let Some(entry) = self.entries.get(index) {
                ctx.event_sender.send_mode_change(ModeKind::RevisionDetails(
                    entry.hash.clone(),
                ));
            }
        } else if let State::Idle = self.state {
            match key {
                Key::Char('g') => {
                    let index = self.select.cursor();
                    if let Some(entry) = self.entries.get(index) {
                        self.state = State::Waiting(WaitOperation::Checkout);
                        let revision = entry.hash.clone();
                        request(ctx, move |b| b.checkout(&revision));
                    }
                }
                Key::Char('m') => {
                    let index = self.select.cursor();
                    if let Some(entry) = self.entries.get(index) {
                        self.state = State::Waiting(WaitOperation::Merge);
                        let revision = entry.hash.clone();
                        request(ctx, move |b| b.merge(&revision));
                    }
                }
                Key::Char('f') => {
                    self.state = State::Waiting(WaitOperation::Fetch);
                    request(ctx, Backend::fetch);
                }
                Key::Char('p') => {
                    self.state = State::Waiting(WaitOperation::Pull);
                    request(ctx, Backend::pull);
                }
                Key::Char('P') => {
                    self.state = State::Waiting(WaitOperation::Push);
                    request(ctx, Backend::push);
                }
                _ => (),
            }
        }

        ModeStatus {
            pending_input: false,
        }
    }

    pub fn on_response(&mut self, response: Response) {
        match response {
            Response::Refresh(result) => {
                self.entries = Vec::new();
                self.output.set(String::new());

                if let State::Waiting(_) = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    match result {
                        Ok(entries) => self.entries = entries,
                        Err(error) => self.output.set(error),
                    }
                }

                self.select.saturate_cursor(self.entries.len());
            }
        }
    }

    pub fn header(&self) -> HeaderInfo {
        match self.state {
            State::Idle => HeaderInfo {
                name: "log",
                waiting_response: false,
            },
            State::Waiting(WaitOperation::Refresh) => HeaderInfo {
                name: "log",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::Checkout) => HeaderInfo {
                name: "checkout",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::Merge) => HeaderInfo {
                name: "merge",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::Fetch) => HeaderInfo {
                name: "fetch",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::Pull) => HeaderInfo {
                name: "pull",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::Push) => HeaderInfo {
                name: "push",
                waiting_response: true,
            },
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        if self.output.text().is_empty() {
            // TODO: toggle full entry
            drawer.select_menu(&self.select, 0, false, self.entries.iter());
        } else {
            drawer.output(&self.output);
        }
    }
}

fn request<F>(ctx: &ModeContext, f: F)
where
    F: 'static + Send + Sync + FnOnce(&dyn Backend) -> BackendResult<()>,
{
    let ctx = ctx.clone();
    thread::spawn(move || {
        use std::ops::Deref;

        let available_height = ctx.viewport_size.1.saturating_sub(1) as _;
        let result = f(ctx.backend.deref())
            .and_then(|_| ctx.backend.log(0, available_height));
        ctx.event_sender
            .send_response(ModeResponse::Log(Response::Refresh(result)));
    });
}

