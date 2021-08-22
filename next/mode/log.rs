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

enum State {
    Idle,
    WaitingForEntries,
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

        drawer.write(&format_args!(
            "{}{} {}{} {}{} {}{} {}{} {}{}",
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

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<LogEntry>,
    output: Output,
    select: SelectMenu,
}
impl Mode {
    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::WaitingForEntries = self.state {
            return;
        }
        self.state = State::WaitingForEntries;

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
        InputStatus { pending: false }

        // c checkout revision
        // U revert revision
        // d revision details
        // f fetch
        // p pull
        // P push
    }

    pub fn on_response(&mut self, response: Response) {
        match response {
            Response::Refresh(entries) => {
                self.entries.clear();
                self.output.set(String::new());

                if let State::WaitingForEntries = self.state {
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
            State::WaitingForEntries => HeaderInfo {
                name: "log",
                waiting_response: true,
            },
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        match self.state {
            State::Idle | State::WaitingForEntries => {
                drawer.select_menu(&self.select, 0, self.entries.iter());
            }
        }
    }
}

