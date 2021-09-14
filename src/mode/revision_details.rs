use std::thread;

use crate::{
    backend::{RevisionEntry, RevisionInfo},
    mode::{Filter, ModeContext, ModeResponse, ModeStatus, Output, SelectMenu, SelectMenuAction},
    platform::Key,
    ui::{Drawer, RESERVED_LINES_COUNT},
};

pub enum Response {
    Info(RevisionInfo),
    Diff(String),
}

enum State {
    Idle,
    Waiting,
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
    entries: Vec<RevisionEntry>,
    output: Output,
    select: SelectMenu,
    filter: Filter,
    show_full_message: bool,
}
impl Mode {
    fn get_selected_entries(&self) -> Vec<RevisionEntry> {
        self.entries
            .iter()
            .filter(|&e| e.selected)
            .cloned()
            .collect()
    }

    pub fn on_enter(&mut self, ctx: &ModeContext, revision: &str) {
        if let State::Waiting = self.state {
            return;
        }
        self.state = State::Waiting;

        self.output.set(String::new());
        self.filter.clear();
        self.select.cursor = 0;
        self.show_full_message = false;

        let ctx = ctx.clone();
        let revision = revision.to_string();
        thread::spawn(move || {
            let mut info = match ctx.backend.revision_details(&revision) {
                Ok(info) => info,
                Err(error) => RevisionInfo {
                    message: error,
                    entries: Vec::new(),
                },
            };
            info.entries
                .sort_unstable_by(|a, b| a.status.cmp(&b.status));

            ctx.event_sender
                .send_response(ModeResponse::RevisionDetails(Response::Info(info)));
        });
    }

    pub fn on_key(&mut self, ctx: &ModeContext, revision: &str, key: Key) -> ModeStatus {
        let pending_input = self.filter.has_focus();
        let available_height = (ctx.viewport_size.1 as usize).saturating_sub(RESERVED_LINES_COUNT);

        if self.filter.has_focus() {
            self.filter.on_key(key);
            self.filter.filter(self.entries.iter());
            self.select
                .saturate_cursor(self.filter.visible_indices().len());
        } else {
            match self.state {
                State::Idle => {
                    let line_count = if self.show_full_message {
                        self.output.line_count()
                    } else {
                        1
                    };

                    match self.select.on_key(
                        self.filter.visible_indices().len(),
                        available_height.saturating_sub(line_count + 1),
                        key,
                    ) {
                        SelectMenuAction::None => (),
                        SelectMenuAction::Toggle(i) => {
                            if let Some(i) = self.filter.get_visible_index(i) {
                                self.entries[i].selected = !self.entries[i].selected
                            }
                        }
                        SelectMenuAction::ToggleAll => {
                            let all_selected = self
                                .filter
                                .visible_indices()
                                .iter()
                                .all(|&i| self.entries[i].selected);
                            for &i in self.filter.visible_indices() {
                                self.entries[i].selected = !all_selected;
                            }
                        }
                    }

                    match key {
                        Key::Ctrl('f') => self.filter.enter(),
                        Key::Tab => {
                            self.show_full_message = !self.show_full_message;
                        }
                        Key::Char('d') => {
                            if !self.entries.is_empty() {
                                self.state = State::ViewDiff;
                                self.output.set(String::new());
                                self.filter.clear();

                                let entries = self.get_selected_entries();

                                let ctx = ctx.clone();
                                let revision = revision.to_string();
                                thread::spawn(move || {
                                    let output = match ctx.backend.diff(Some(&revision), &entries) {
                                        Ok(output) => output,
                                        Err(error) => error,
                                    };
                                    ctx.event_sender
                                        .send_response(ModeResponse::RevisionDetails(
                                            Response::Diff(output),
                                        ));
                                });
                            }
                        }
                        _ => (),
                    }
                }
                State::ViewDiff => self.output.on_key(available_height, key),
                _ => (),
            }
        }

        ModeStatus { pending_input }
    }

    pub fn on_response(&mut self, response: Response) {
        match response {
            Response::Info(info) => {
                if let State::Waiting = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    self.output.set(info.message);
                }

                self.entries = info.entries;

                self.filter.filter(self.entries.iter());
                self.select
                    .saturate_cursor(self.filter.visible_indices().len());
            }
            Response::Diff(mut output) => {
                if let State::ViewDiff = self.state {
                    if output.is_empty() {
                        output.push('\n');
                    }
                    self.output.set(output);
                }
            }
        }
    }

    pub fn is_waiting_response(&self) -> bool {
        match self.state {
            State::Idle => false,
            State::Waiting => true,
            State::ViewDiff => self.output.text().is_empty(),
        }
    }

    pub fn header(&self) -> (&str, &str, &str) {
        match self.state {
            State::Idle | State::Waiting => (
                "revision details",
                "[d]diff",
                "[tab]full message [arrows]move [space]toggle [a]toggle all [ctrl+f]filter",
            ),
            State::ViewDiff => ("diff", "", "[arrows]move"),
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        let filter_line_count = drawer.filter(&self.filter);

        let line_count = if let State::ViewDiff = self.state {
            drawer.diff(&self.output)
        } else if self.show_full_message {
            drawer.output(&self.output)
        } else {
            let output = self.output.text().lines().next().unwrap_or("");
            let output = match output
                .char_indices()
                .nth(drawer.viewport_size.0.saturating_sub(1) as _)
            {
                Some((i, c)) => &output[..i + c.len_utf8()],
                None => output,
            };
            drawer.str(output);
            drawer.next_line();
            1
        };

        let line_count = filter_line_count + line_count;

        if let State::Idle = self.state {
            drawer.next_line();
            drawer.select_menu(
                &self.select,
                (line_count + 1).min(u16::MAX as _) as _,
                false,
                self.filter
                    .visible_indices()
                    .iter()
                    .map(|&i| &self.entries[i]),
            );
        }
    }
}
