use std::thread;

use crate::{
    application::Key,
    backend::{BackendResult, LogEntry},
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

        let mut chars = self.author.char_indices();
        let author = match chars.nth(12) {
            Some((i, c)) => &self.author[..i + c.len_utf8()],
            None => &self.author,
        };

        if self.refs.is_empty() {
            drawer.write(&format_args!(
                "{}{} {}{} {}{} {}{} {}{}",
                color(Color::White, hovered),
                &self.graph,
                color(Color::Yellow, hovered),
                &self.hash,
                color(Color::Blue, hovered),
                &self.date,
                color(Color::Green, hovered),
                author,
                color(Color::White, hovered),
                &self.message,
            ));
        } else {
            drawer.write(&format_args!(
                "{}{} {}{} {}{} {}{} {}({}) {}{}",
                color(Color::White, hovered),
                &self.graph,
                color(Color::Yellow, hovered),
                &self.hash,
                color(Color::Blue, hovered),
                &self.date,
                color(Color::Green, hovered),
                author,
                color(Color::Red, hovered),
                &self.refs,
                color(Color::White, hovered),
                &self.message,
            ));
        }
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
        match self.state {
            State::Idle | State::WaitingForEntries(_) => {
                let available_height =
                    ctx.viewport_size.1.saturating_sub(1) as usize;
                self.select
                    .on_key(self.entries.len(), available_height, key);

                match key {
                    Key::Char('c') => {
                        // checkout revision
                    }
                    Key::Char('d') => {
                        // revision details
                    }
                    Key::Char('f') => {
                        if let State::Idle = self.state {
                            self.state =
                                State::WaitingForEntries(WaitOperation::Fetch);

                            let ctx = ctx.clone();
                            thread::spawn(move || {
                                let result =
                                    ctx.backend.fetch().and_then(|_| {
                                        ctx.backend.log(0, available_height)
                                    });
                                ctx.response_sender.send(ModeResponse::Log(
                                    Response::Refresh(result),
                                ));
                            });
                        }
                    }
                    Key::Char('p') => {
                        if let State::Idle = self.state {
                            self.state =
                                State::WaitingForEntries(WaitOperation::Pull);

                            let ctx = ctx.clone();
                            thread::spawn(move || {
                                let result =
                                    ctx.backend.pull().and_then(|_| {
                                        ctx.backend.log(0, available_height)
                                    });
                                ctx.response_sender.send(ModeResponse::Log(
                                    Response::Refresh(result),
                                ));
                            });
                        }
                    }
                    Key::Char('P') => {
                        if let State::Idle = self.state {
                            self.state =
                                State::WaitingForEntries(WaitOperation::Push);

                            let ctx = ctx.clone();
                            thread::spawn(move || {
                                let result =
                                    ctx.backend.push().and_then(|_| {
                                        ctx.backend.log(0, available_height)
                                    });
                                ctx.response_sender.send(ModeResponse::Log(
                                    Response::Refresh(result),
                                ));
                            });
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

