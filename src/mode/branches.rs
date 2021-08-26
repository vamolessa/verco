use std::thread;

use crate::{
    application::Key,
    backend::{Backend, BackendResult, BranchEntry},
    mode::{
        HeaderInfo, ModeContext, ModeKind, ModeResponse, ModeStatus, Output,
        ReadLine, SelectMenu,
    },
    ui::{Drawer, SelectEntryDraw},
};

pub enum Response {
    Refresh(BackendResult<Vec<BranchEntry>>),
    Checkout,
    Merge,
}

enum WaitOperation {
    Refresh,
    New,
    Delete,
    Merge,
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

impl SelectEntryDraw for BranchEntry {
    fn draw(&self, drawer: &mut Drawer, _: bool, _: bool) -> usize {
        let status = if self.checked_out {
            " (checked out)"
        } else {
            ""
        };
        drawer.fmt(format_args!("{}{}", self.name, status));
        1
    }
}

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<BranchEntry>,
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
                    Key::Char('g') => {
                        let index = self.select.cursor();
                        if let Some(entry) = self.entries.get(index) {
                            let name = entry.name.clone();
                            let ctx = ctx.clone();
                            thread::spawn(move || {
                                ctx.event_sender
                                    .send_mode_change(ModeKind::Log);
                                match ctx.backend.checkout(&name) {
                                    Ok(()) => {
                                        ctx.event_sender.send_response(
                                            ModeResponse::Branches(
                                                Response::Checkout,
                                            ),
                                        );
                                        ctx.event_sender
                                            .send_mode_refresh(ModeKind::Log);
                                    }
                                    Err(error) => ctx
                                        .event_sender
                                        .send_response(ModeResponse::Branches(
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
                            self.select.on_remove_entry(index);
                            request(ctx, move |b| b.delete_branch(&name));
                        }
                    }
                    Key::Char('m') => {
                        let index = self.select.cursor();
                        if let Some(entry) = self.entries.get(index) {
                            self.state = State::Waiting(WaitOperation::Merge);

                            let name = entry.name.clone();
                            let ctx = ctx.clone();
                            thread::spawn(move || {
                                ctx.event_sender
                                    .send_mode_change(ModeKind::Log);
                                match ctx.backend.merge(&name) {
                                    Ok(()) => {
                                        ctx.event_sender.send_response(
                                            ModeResponse::Branches(
                                                Response::Merge,
                                            ),
                                        );
                                        ctx.event_sender
                                            .send_mode_refresh(ModeKind::Log);
                                    }
                                    Err(error) => ctx
                                        .event_sender
                                        .send_response(ModeResponse::Branches(
                                            Response::Refresh(Err(error)),
                                        )),
                                }
                            });
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
                    request(ctx, move |b| b.new_branch(&name));
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

                if let Some(i) = self.entries.iter().position(|e| e.checked_out)
                {
                    self.select.set_cursor(i);
                } else {
                    self.select.saturate_cursor(self.entries.len());
                }
            }
            Response::Checkout | Response::Merge => self.state = State::Idle,
        }
    }

    pub fn header(&self) -> HeaderInfo {
        match self.state {
            State::Idle => HeaderInfo {
                name: "branches",
                waiting_response: false,
            },
            State::Waiting(WaitOperation::Refresh) => HeaderInfo {
                name: "branches",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::New) => HeaderInfo {
                name: "new branch",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::Delete) => HeaderInfo {
                name: "delete branch",
                waiting_response: true,
            },
            State::Waiting(WaitOperation::Merge) => HeaderInfo {
                name: "merge branch",
                waiting_response: true,
            },
            State::NewNameInput => HeaderInfo {
                name: "new branch name",
                waiting_response: false,
            },
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        match self.state {
            State::Idle | State::Waiting(_) => {
                if self.output.text.is_empty() {
                    drawer.select_menu(&self.select, 0, false, self.entries.iter());
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
            f(ctx.backend.deref()).and_then(|_| ctx.backend.branches());
        if let Ok(entries) = &mut result {
            entries.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        }

        ctx.event_sender
            .send_response(ModeResponse::Branches(Response::Refresh(result)));
    });
}

