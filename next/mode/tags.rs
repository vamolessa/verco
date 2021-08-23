use std::thread;

use crate::{
    application::Key,
    backend::{Backend, BackendResult, TagEntry},
    mode::{
        HeaderInfo, ModeContext, ModeKind, ModeResponse, ModeStatus, Output,
        ReadLine, SelectMenu,
    },
    ui::{Drawer, SelectEntryDraw},
};

pub enum Response {
    Refresh(BackendResult<Vec<TagEntry>>),
    Checkout,
}

enum WaitOperation {
    Refresh,
    New,
    Delete,
}

enum State {
    Idle,
    Waiting(WaitOperation),
    NewNameInput,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl SelectEntryDraw for TagEntry {
    fn draw(&self, drawer: &mut Drawer, _: bool) {
        drawer.write(&self.name);
    }
}

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<TagEntry>,
    output: Output,
    select: SelectMenu,
    readline: ReadLine,
}
impl Mode {
    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::Waiting(_) = self.state {
            return;
        }
        self.state = State::Waiting(WaitOperation::Refresh);

        self.output.set(String::new());
        self.readline.clear();

        request(ctx, |_| Ok(()));
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> ModeStatus {
        let pending_input = matches!(self.state, State::NewNameInput);
        let available_height = ctx.viewport_size.1.saturating_sub(1) as usize;

        match self.state {
            State::Idle | State::Waiting(_) => {
                if self.output.text().is_empty() {
                    self.select.on_key(
                        self.entries.len(),
                        available_height,
                        key,
                    );
                } else {
                    self.output.on_key(available_height, key);
                }

                match key {
                    Key::Char('c') => {
                        let index = self.select.cursor();
                        if let Some(entry) = self.entries.get(index) {
                            let name = entry.name.clone();
                            let ctx = ctx.clone();
                            thread::spawn(move || {
                                match ctx.backend.checkout(&name) {
                                    Ok(()) => {
                                        ctx.event_sender.send_response(
                                            ModeResponse::Tags(
                                                Response::Checkout,
                                            ),
                                        );
                                        ctx.event_sender
                                            .send_mode_change(ModeKind::Log);
                                    }
                                    Err(error) => ctx
                                        .event_sender
                                        .send_response(ModeResponse::Tags(
                                            Response::Refresh(Err(error)),
                                        )),
                                }
                            });
                        }
                    }
                    Key::Char('n') => {
                        self.state = State::NewNameInput;
                        self.output.set(String::new());
                        self.readline.clear();
                    }
                    Key::Char('D') => {
                        let index = self.select.cursor();
                        if let Some(entry) = self.entries.get(index) {
                            self.state = State::Waiting(WaitOperation::Delete);

                            let name = entry.name.clone();
                            self.entries.remove(index);
                            request(ctx, move |b| b.delete_tag(&name));
                        }
                    }
                    _ => (),
                }
            }
            State::NewNameInput => {
                self.readline.on_key(key);
                if key.is_submit() {
                    self.state = State::Waiting(WaitOperation::New);

                    let name = self.readline.input().to_string();
                    request(ctx, move |b| b.new_tag(&name));
                }
            }
        }

        ModeStatus { pending_input }
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
            Response::Checkout => self.state = State::Idle,
        }
    }

    pub fn header(&self) -> HeaderInfo {
        match self.state {
            State::Idle => HeaderInfo {
                name: "tags",
                waiting_response: false,
            },
            State::Waiting(WaitOperation::Refresh) => HeaderInfo {
                name: "tags",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::New) => HeaderInfo {
                name: "new tag",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::Delete) => HeaderInfo {
                name: "delete tag",
                waiting_response: true,
            },
            State::NewNameInput => HeaderInfo {
                name: "new tag name",
                waiting_response: false,
            },
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        match self.state {
            State::Idle | State::Waiting(_) => {
                if self.output.text.is_empty() {
                    drawer.select_menu(&self.select, 0, self.entries.iter());
                } else {
                    drawer.output(&self.output);
                }
            }
            State::NewNameInput => drawer.readline(&self.readline),
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

        let mut result =
            f(ctx.backend.deref()).and_then(|_| ctx.backend.tags());
        if let Ok(entries) = &mut result {
            entries.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        }

        ctx.event_sender
            .send_response(ModeResponse::Tags(Response::Refresh(result)));
    });
}
