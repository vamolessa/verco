use std::thread;

use crate::{
    application::Key,
    backend::BackendResult,
    mode::{
        ModeContext, ModeResponse, ReadLine, ReadLineAction, SelectMenu,
        SelectMenuAction,
    },
    ui,
};

#[derive(Clone)]
struct FileEntry {
    pub selected: bool,
}

pub enum Response {
    Entries(BackendResult<Vec<FileEntry>>),
}

enum State {
    Idle,
    WaitingForEntries,
    CommitMessageInput,
    ViewCommitResult,
    ViewRevertResult,
    ViewDiff,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<FileEntry>,
    message: String,
    select: SelectMenu,
    readline: ReadLine,
}
impl Mode {
    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::WaitingForEntries = self.state {
            return;
        }
        self.state = State::WaitingForEntries;

        let ctx = ctx.clone();
        thread::spawn(move || {
            let response = match ctx.backend.status() {
                BackendResult::Ok(_) => BackendResult::Ok(Vec::new()),
                BackendResult::Err(error) => BackendResult::Err(error),
            };
            let response = Response::Entries(response);
            ctx.response_sender.send(ModeResponse::Status(response));
        });
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> bool {
        let available_height = ctx.viewport_size.1.saturating_sub(1) as usize;

        match self.state {
            State::Idle | State::WaitingForEntries => {
                match self.select.on_key(
                    self.entries.len(),
                    available_height,
                    key,
                ) {
                    SelectMenuAction::None => (),
                    SelectMenuAction::Toggle(i) => {
                        self.entries[i].selected = !self.entries[i].selected
                    }
                    SelectMenuAction::ToggleAll => {
                        let all_selected =
                            self.entries.iter().all(|e| e.selected);
                        for entry in &mut self.entries {
                            entry.selected = !all_selected;
                        }
                    }
                }

                match key {
                    Key::Char('c') => {
                        self.state = State::CommitMessageInput;
                        // TODO: goto commit
                    }
                    Key::Char('U') => {
                        // TODO: goto revert
                    }
                    Key::Char('d') => {
                        // TODO: goto diff
                    }
                    _ => (),
                }
            }
            State::CommitMessageInput => {
                match self.readline.on_key(key) {
                    ReadLineAction::None => (),
                    ReadLineAction::Submit => {
                        self.state = State::ViewCommitResult;
                        // TODO: send request
                    }
                    ReadLineAction::Cancel => self.on_enter(ctx),
                }
            }
            _ => {
                //
            }
        }

        matches!(self.state, State::CommitMessageInput)
    }

    pub fn on_response(&mut self, response: Response) {
        self.message.clear();

        match response {
            Response::Entries(entries) => {
                if let State::WaitingForEntries = self.state {
                    self.state = State::Idle;
                }
                match entries {
                    Ok(entries) => self.entries = entries.clone(),
                    Err(error) => self.message.push_str(&error),
                }
            }
        }
    }

    pub fn draw(&self, viewport_size: (u16, u16)) {
        match self.state {
            State::Idle => {
                //ui::draw_output(self.name(), &self.output.lock().unwrap());
            }
            State::WaitingForEntries => {
                //
            }
            State::CommitMessageInput => (),
            State::ViewCommitResult => (),
            State::ViewRevertResult => (),
            State::ViewDiff => (),
        }
    }
}

