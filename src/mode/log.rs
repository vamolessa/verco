use std::thread;

use crate::{
    backend::{Backend, BackendResult, LogEntry},
    mode::{
        ModeContext, ModeKind, ModeResponse, ModeStatus, Output, SelectMenu,
    },
    platform::Key,
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

        let (line_count, message) = if full {
            let mut line_count = 0;
            for line in self.message.lines() {
                let mut x = 0;
                for _ in line.chars() {
                    if x >= drawer.viewport_size.0 as _ {
                        x -= drawer.viewport_size.0 as usize;
                        line_count += 1;
                    }
                }

                line_count += 1;
            }
            (line_count, &self.message[..])
        } else {
            let available_width =
                (drawer.viewport_size.0 as usize).saturating_sub(total_chars);
            let message = match self.message.char_indices().nth(available_width)
            {
                Some((i, _)) => &self.message[..i],
                None => &self.message,
            };
            (0, message)
        };

        let (refs_begin, refs_end) = match &self.refs[..] {
            "" => ("", ""),
            _ => ("(", ") "),
        };

        drawer.fmt(format_args!(
            "{}{} {}{} {}{} {}{} {}{}{}{}{}",
            color(Color::White, hovered),
            &self.graph,
            color(Color::DarkYellow, hovered),
            &self.hash,
            color(Color::DarkBlue, hovered),
            &self.date,
            color(Color::DarkGreen, hovered),
            author,
            color(Color::DarkRed, hovered),
            refs_begin,
            &self.refs,
            refs_end,
            color(Color::White, hovered),
        ));

        if full {
            drawer.next_line();
        }

        drawer.str(message);

        1 + line_count
    }
}

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<LogEntry>,
    output: Output,
    select: SelectMenu,
    show_full_hovered_message: bool,
}
impl Mode {
    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::Waiting(_) = self.state {
            return;
        }
        self.state = State::Waiting(WaitOperation::Refresh);

        self.output.set(String::new());
        self.show_full_hovered_message = false;

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
        } else if let Key::Tab = key {
            self.show_full_hovered_message = !self.show_full_hovered_message;
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

    pub fn is_waiting_response(&self) -> bool {
        match self.state {
            State::Idle => false,
            State::Waiting(_) => true,
        }
    }

    pub fn header(&self) -> &str {
        match self.state {
            State::Idle => "log",
            State::Waiting(WaitOperation::Refresh) => "log",
            State::Waiting(WaitOperation::Checkout) => "checkout",
            State::Waiting(WaitOperation::Merge) => "merge",
            State::Waiting(WaitOperation::Fetch) => "fetch",
            State::Waiting(WaitOperation::Pull) => "pull",
            State::Waiting(WaitOperation::Push) => "push",
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        if self.output.text().is_empty() {
            drawer.select_menu(
                &self.select,
                0,
                self.show_full_hovered_message,
                self.entries.iter(),
            );
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
