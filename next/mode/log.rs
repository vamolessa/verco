use std::thread;

use crate::{
    application::Key,
    backend::{Backend, BackendResult, LogEntry},
    mode::{
        HeaderInfo, InputStatus, ModeContext, ModeResponse, Output, SelectMenu,
    },
    ui::{Color, Drawer, SelectEntryDraw},
};

pub enum Response {
    Refresh(BackendResult<Vec<LogEntry>>),
}

enum WaitOperation {
    None,
    Checkout,
    Fetch,
    Pull,
    Push,
}

enum State {
    Idle,
    WaitingForEntries(WaitOperation),
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl SelectEntryDraw for LogEntry {
    fn draw(&self, drawer: &mut Drawer, hovered: bool) {
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
        if let State::WaitingForEntries(_) = self.state {
            return;
        }
        self.state = State::WaitingForEntries(WaitOperation::None);

        self.output.set(String::new());

        let ctx = ctx.clone();
        thread::spawn(move || {
            let entries = ctx
                .backend
                .log(0, ctx.viewport_size.1.saturating_sub(1) as _);
            ctx.response_sender
                .send(ModeResponse::Log(Response::Refresh(entries)));
        });
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> InputStatus {
        fn request<F>(ctx: &ModeContext, available_height: usize, f: F)
        where
            F: 'static
                + Send
                + Sync
                + FnOnce(&dyn Backend) -> BackendResult<()>,
        {
            let ctx = ctx.clone();
            thread::spawn(move || {
                use std::ops::Deref;
                let result = f(ctx.backend.deref())
                    .and_then(|_| ctx.backend.log(0, available_height));
                ctx.response_sender
                    .send(ModeResponse::Log(Response::Refresh(result)));
            });
        }

        match self.state {
            State::Idle | State::WaitingForEntries(_) => {
                let available_height =
                    ctx.viewport_size.1.saturating_sub(1) as usize;
                self.select
                    .on_key(self.entries.len(), available_height, key);

                match key {
                    Key::Char('c') => {
                        let index = self.select.cursor();
                        if let (State::Idle, Some(entry)) =
                            (&self.state, self.entries.get(index))
                        {
                            self.state = State::WaitingForEntries(
                                WaitOperation::Checkout,
                            );
                            let revision = entry.hash.clone();
                            request(ctx, available_height, move |b| {
                                b.checkout(&revision)
                            });
                        }
                    }
                    Key::Char('d') => {
                        // revision details
                    }
                    Key::Char('f') => {
                        if let State::Idle = self.state {
                            self.state =
                                State::WaitingForEntries(WaitOperation::Fetch);
                            request(ctx, available_height, Backend::fetch);
                        }
                    }
                    Key::Char('p') => {
                        if let State::Idle = self.state {
                            self.state =
                                State::WaitingForEntries(WaitOperation::Pull);
                            request(ctx, available_height, Backend::pull);
                        }
                    }
                    Key::Char('P') => {
                        if let State::Idle = self.state {
                            self.state =
                                State::WaitingForEntries(WaitOperation::Push);
                            request(ctx, available_height, Backend::push);
                        }
                    }
                    _ => (),
                }
            }
        }

        InputStatus { pending: false }
    }

    pub fn on_response(&mut self, response: Response) {
        match response {
            Response::Refresh(entries) => {
                self.entries.clear();
                self.output.set(String::new());

                if let State::WaitingForEntries(_) = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    match entries {
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
            State::WaitingForEntries(WaitOperation::None) => HeaderInfo {
                name: "log",
                waiting_response: true,
            },
            State::WaitingForEntries(WaitOperation::Checkout) => HeaderInfo {
                name: "checkout",
                waiting_response: true,
            },
            State::WaitingForEntries(WaitOperation::Fetch) => HeaderInfo {
                name: "fetch",
                waiting_response: true,
            },
            State::WaitingForEntries(WaitOperation::Pull) => HeaderInfo {
                name: "pull",
                waiting_response: true,
            },
            State::WaitingForEntries(WaitOperation::Push) => HeaderInfo {
                name: "push",
                waiting_response: true,
            },
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        match self.state {
            State::Idle | State::WaitingForEntries(_) => {
                drawer.select_menu(&self.select, 0, self.entries.iter());
            }
        }
    }
}

