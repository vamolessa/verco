use std::thread;

use crate::{
    backend::{Backend, BackendResult, FileStatus, RevisionEntry, StatusInfo},
    mode::{
        Filter, ModeContext, ModeKind, ModeResponse, ModeStatus, Output, ReadLine, SelectMenu,
        SelectMenuAction,
    },
    platform::Key,
    ui::{Color, Drawer, SelectEntryDraw, RESERVED_LINES_COUNT},
};

pub enum Response {
    Refresh(StatusInfo),
    Commit,
    Diff(String),
}

enum WaitOperation {
    Refresh,
    Commit,
    Discard,
    ResolveTakingOurs,
    ResolveTakingTheirs,
}

enum State {
    Idle,
    Waiting(WaitOperation),
    CommitMessageInput,
    ViewDiff,
}
impl Default for State {
    fn default() -> Self {
        Self::Idle
    }
}

impl SelectEntryDraw for RevisionEntry {
    fn draw(&self, drawer: &mut Drawer, _: bool, _: bool) -> usize {
        const NAME_TOO_LONG_PREFIX: &str = "...";

        let name_available_width = (drawer.viewport_size.0 as usize)
            .saturating_sub(2 + 1 + FileStatus::max_len() + 1 + 1 + NAME_TOO_LONG_PREFIX.len() + 1);

        let (name_prefix, trimmed_name) =
            match self.name.char_indices().nth_back(name_available_width) {
                Some((i, _)) => (NAME_TOO_LONG_PREFIX, &self.name[i..]),
                None => ("", &self.name[..]),
            };

        let selected_text = if self.selected { '+' } else { ' ' };
        drawer.fmt(format_args!(
            "{} [{:>width$}] {}{}",
            selected_text,
            self.status.as_str(),
            name_prefix,
            trimmed_name,
            width = FileStatus::max_len(),
        ));

        1
    }
}

#[derive(Default)]
pub struct Mode {
    state: State,
    entries: Vec<RevisionEntry>,
    output: Output,
    select: SelectMenu,
    filter: Filter,
    readline: ReadLine,
}
impl Mode {
    fn get_selected_entries(&self) -> Vec<RevisionEntry> {
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|&e| e.selected)
            .cloned()
            .collect();
        entries
    }

    fn remove_selected_entries(&mut self) {
        let previous_len = self.entries.len();

        for i in (0..self.entries.len()).rev() {
            if self.entries[i].selected {
                self.entries.remove(i);
                self.filter.on_remove_entry(i);
                let i = match self.filter.visible_indices().binary_search(&i) {
                    Ok(i) => i,
                    Err(i) => i,
                };
                self.select.on_remove_entry(i);
            }
        }

        if self.entries.len() == previous_len {
            self.entries.clear();
            self.select.cursor = 0;
            self.filter.clear();
        }
    }

    pub fn on_enter(&mut self, ctx: &ModeContext) {
        if let State::Waiting(_) = self.state {
            return;
        }
        self.state = State::Waiting(WaitOperation::Refresh);

        self.output.set(String::new());
        let cursor = self.filter.filter(self.entries.iter(), self.select.cursor);
        let available_height = (ctx.viewport_size.1 as usize).saturating_sub(RESERVED_LINES_COUNT);
        self.select.fix_cursor_on_filter(cursor, available_height);
        self.readline.clear();

        request(ctx, |_| Ok(()));
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> ModeStatus {
        let pending_input =
            matches!(self.state, State::CommitMessageInput) || self.filter.has_focus();
        let available_height = (ctx.viewport_size.1 as usize).saturating_sub(RESERVED_LINES_COUNT);

        if self.filter.has_focus() {
            self.filter.on_key(key);
            let cursor = self.filter.filter(self.entries.iter(), self.select.cursor);
            self.select.fix_cursor_on_filter(cursor, available_height);
        } else {
            match self.state {
                State::Idle | State::Waiting(_) => {
                    if self.output.line_count() > 1 {
                        self.output.on_key(available_height, key);
                    } else {
                        match self.select.on_key(
                            self.filter.visible_indices().len(),
                            available_height.saturating_sub(2),
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
                    }

                    match key {
                        Key::Ctrl('f') => self.filter.enter(),
                        Key::Char('c') => {
                            if !self.entries.is_empty() {
                                self.state = State::CommitMessageInput;
                                self.output.set(String::new());
                                self.filter.clear();
                                self.readline.clear();
                            }
                        }
                        Key::Char('R') => {
                            if matches!(self.state, State::Idle) && !self.entries.is_empty() {
                                self.state = State::Waiting(WaitOperation::Discard);
                                let entries = self.get_selected_entries();
                                self.remove_selected_entries();

                                request(ctx, move |b| b.discard(&entries));
                            }
                        }
                        Key::Char('O') => {
                            if matches!(self.state, State::Idle) && !self.entries.is_empty() {
                                self.state = State::Waiting(WaitOperation::ResolveTakingOurs);
                                let entries = self.get_selected_entries();

                                request(ctx, move |b| b.resolve_taking_ours(&entries));
                            }
                        }
                        Key::Char('T') => {
                            if matches!(self.state, State::Idle) && !self.entries.is_empty() {
                                self.state = State::Waiting(WaitOperation::ResolveTakingTheirs);
                                let entries = self.get_selected_entries();

                                request(ctx, move |b| b.resolve_taking_theirs(&entries));
                            }
                        }
                        Key::Char('d') => {
                            if !self.entries.is_empty() {
                                self.state = State::ViewDiff;
                                self.output.set(String::new());
                                self.filter.clear();

                                let entries = self.get_selected_entries();

                                let ctx = ctx.clone();
                                thread::spawn(move || {
                                    let output = match ctx.backend.diff(None, &entries) {
                                        Ok(output) => output,
                                        Err(error) => error,
                                    };
                                    ctx.event_sender.send_response(ModeResponse::Status(
                                        Response::Diff(output),
                                    ));
                                });
                            }
                        }
                        _ => (),
                    }
                }
                State::CommitMessageInput => {
                    self.readline.on_key(key);
                    if key.is_submit() {
                        self.state = State::Waiting(WaitOperation::Commit);

                        let message = self.readline.input().to_string();
                        let entries = self.get_selected_entries();
                        self.remove_selected_entries();

                        let ctx = ctx.clone();
                        thread::spawn(move || {
                            ctx.event_sender.send_mode_change(ModeKind::Log);
                            match ctx.backend.commit(&message, &entries) {
                                Ok(()) => {
                                    ctx.event_sender
                                        .send_response(ModeResponse::Status(Response::Commit));
                                    ctx.event_sender.send_mode_refresh(ModeKind::Log);
                                }
                                Err(error) => ctx.event_sender.send_response(ModeResponse::Status(
                                    Response::Refresh(StatusInfo {
                                        header: error,
                                        entries: Vec::new(),
                                    }),
                                )),
                            }
                        });
                    } else if key.is_cancel() {
                        self.on_enter(ctx);
                    }
                }
                _ => self.output.on_key(available_height, key),
            }
        }

        ModeStatus { pending_input }
    }

    pub fn on_response(&mut self, ctx: &ModeContext, response: Response) {
        match response {
            Response::Refresh(info) => {
                if let State::Waiting(_) = self.state {
                    self.state = State::Idle;
                }
                if let State::Idle = self.state {
                    self.output.set(info.header);
                }

                self.entries = info.entries;

                let cursor = self.filter.filter(self.entries.iter(), self.select.cursor);
                let available_height = (ctx.viewport_size.1 as usize).saturating_sub(RESERVED_LINES_COUNT);
                self.select.fix_cursor_on_filter(cursor, available_height);
            }
            Response::Commit => self.state = State::Idle,
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
            State::Idle | State::CommitMessageInput => false,
            State::Waiting(_) => true,
            State::ViewDiff => self.output.text().is_empty(),
        }
    }

    pub fn header(&self) -> (&str, &str, &str) {
        let name = match self.state {
            State::Idle | State::Waiting(WaitOperation::Refresh) => "status",
            State::CommitMessageInput => "commit message",
            State::Waiting(WaitOperation::Commit) => "commit",
            State::Waiting(WaitOperation::Discard) => "discard",
            State::Waiting(WaitOperation::ResolveTakingOurs) => "resolve taking ours",
            State::Waiting(WaitOperation::ResolveTakingTheirs) => "resolve taking theirs",
            State::ViewDiff => "diff",
        };
        let (left_help, right_help) = match self.state {
            State::Idle | State::Waiting(_) => (
                "[c]commit [R]revert [d]diff [O]take ours [T]take theirs",
                "[arrows]move [space]toggle [a]toggle all [ctrl+f]filter",
            ),
            State::CommitMessageInput => (
                "",
                "[enter]submit [esc]cancel [ctrl+w]delete word [ctrl+u]delete all",
            ),
            State::ViewDiff => ("", "[arrows]move"),
        };
        (name, left_help, right_help)
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        let filter_line_count = drawer.filter(&self.filter);
        match self.state {
            State::Idle | State::Waiting(_) => {
                if self.output.line_count() > 1 {
                    drawer.output(&self.output);
                } else {
                    let output = self.output.text();
                    let output = match output
                        .char_indices()
                        .nth((drawer.viewport_size.0 as usize).saturating_sub(RESERVED_LINES_COUNT))
                    {
                        Some((i, c)) => &output[..i + c.len_utf8()],
                        None => output,
                    };

                    drawer.str(output);
                    drawer.next_line();
                    drawer.next_line();
                    drawer.select_menu(
                        &self.select,
                        2 + filter_line_count,
                        false,
                        self.filter
                            .visible_indices()
                            .iter()
                            .map(|&i| &self.entries[i]),
                    );

                    if self.entries.is_empty() {
                        let empty_message = match self.state {
                            State::Idle => "nothing to commit!",
                            _ => "working...",
                        };
                        drawer.fmt(format_args!("{}{}", Color::DarkYellow, empty_message));
                    }
                }
            }
            State::CommitMessageInput => {
                drawer.readline(&self.readline, "type in the commit message...")
            }
            State::ViewDiff => {
                drawer.diff(&self.output);
            }
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

        let mut info = match f(ctx.backend.deref()).and_then(|_| ctx.backend.status()) {
            Ok(info) => info,
            Err(error) => StatusInfo {
                header: error,
                entries: Vec::new(),
            },
        };
        info.entries
            .sort_unstable_by(|a, b| a.status.cmp(&b.status));

        ctx.event_sender
            .send_response(ModeResponse::Status(Response::Refresh(info)));
    });
}
