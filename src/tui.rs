use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Print, ResetColor, SetForegroundColor},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
    },
    ExecutableCommand, QueueableCommand, Result,
};

use std::{
    io::{stdout, Write},
    iter,
};

use crate::{
    custom_actions::CustomAction,
    input,
    scroll_view::ScrollView,
    select::{select, Entry},
    tui_util::{show_header, Header, HeaderKind, ENTRY_COLOR},
    version_control_actions::VersionControlActions,
    worker::{Worker, CommandTaskResult},
};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn show_tui(
    version_control: Box<dyn 'static + VersionControlActions>,
    custom_actions: Vec<CustomAction>,
) {
    Tui::new(version_control, custom_actions, stdout().lock())
        .show()
        .unwrap();
}

enum HandleChordResult {
    Handled,
    Unhandled,
    Quit,
}

enum CommandId {
    SomeCommandId,
}

struct Tui<W>
where
    W: Write,
{
    version_control: Box<dyn 'static + VersionControlActions>,
    custom_actions: Vec<CustomAction>,

    current_key_chord: Vec<char>,

    write: W,
    scroll_view: ScrollView,

    worker: Worker<CommandId, CommandTaskResult>
}

impl<W> Tui<W>
where
    W: Write,
{
    fn new(
        version_control: Box<dyn 'static + VersionControlActions>,
        custom_actions: Vec<CustomAction>,
        write: W,
    ) -> Self {
        Tui {
            version_control,
            custom_actions,
            current_key_chord: Vec::new(),
            write,
            scroll_view: Default::default(),
            worker: Worker::new(),
        }
    }

    fn show_header(&mut self, header: &Header, kind: HeaderKind) -> Result<()> {
        show_header(&mut self.write, header, kind).map(|_| ())
    }

    fn show_select_ui(&mut self, entries: &mut Vec<Entry>) -> Result<bool> {
        select(&mut self.write, entries)
    }

    fn action_context<F>(
        &mut self,
        action_name: &str,
        callback: F,
    ) -> Result<HandleChordResult>
    where
        F: FnOnce(&mut Self, &Header) -> Result<()>,
    {
        let header = Header {
            action_name,
            directory_name: self.version_control.get_root().into(),
        };
        show_header(&mut self.write, &header, HeaderKind::Waiting)?;
        self.write.flush()?;
        callback(self, &header).map(|_| HandleChordResult::Handled)
    }

    fn show(&mut self) -> Result<()> {
        execute!(self.write, EnterAlternateScreen, cursor::Hide)?;
        terminal::enable_raw_mode()?;

        self.action_context("help", |s, h| {
            let result = s.show_help(h)?;
            s.handle_result(h, result)
        })?;
        let (w, h) = terminal::size()?;
        queue!(
            self.write,
            cursor::MoveTo(w, h - 1),
            Clear(ClearType::CurrentLine),
        )?;

        loop {
            self.write.flush()?;

            if let Some((command_id, result)) = self.worker.receive_result() {
            }

            match input::read_key()? {
                KeyEvent {
                    code: KeyCode::Esc, ..
                }
                | KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    if self.current_key_chord.len() == 0 {
                        break;
                    }

                    self.current_key_chord.clear();
                    self.show_current_key_chord()?;
                }
                key_event => {
                    if self.scroll_view.update(&mut self.write, &key_event)? {
                        continue;
                    }

                    if let Some(c) = input::key_to_char(key_event) {
                        self.current_key_chord.push(c);
                    }
                    match self.handle_action()? {
                        HandleChordResult::Handled => {
                            self.current_key_chord.clear()
                        }
                        HandleChordResult::Unhandled => (),
                        HandleChordResult::Quit => break,
                    }
                    self.show_current_key_chord()?;
                }
            }
        }

        execute!(
            self.write,
            ResetColor,
            cursor::MoveTo(0, h),
            Clear(ClearType::CurrentLine),
            cursor::Show
        )?;
        terminal::disable_raw_mode()?;
        self.write.execute(LeaveAlternateScreen)?;
        Ok(())
    }

    fn handle_action(&mut self) -> Result<HandleChordResult> {
        match &self.current_key_chord[..] {
            ['q'] => Ok(HandleChordResult::Quit),
            ['h'] => self.action_context("help", |s, h| {
                let result = s.show_help(h)?;
                s.handle_result(h, result)
            }),
            ['s'] => self.action_context("status", |s, h| {
                let result = s.version_control.status();
                s.handle_result(h, result)
            }),
            ['l'] => Ok(HandleChordResult::Unhandled),
            ['l', 'l'] => self.action_context("log", |s, h| {
                let result = s.version_control.log(50);
                s.handle_result(h, result)
            }),
            ['l', 'c'] => self.action_context("log count", |s, h| {
                if let Some(input) = s.handle_input("logs to show (ctrl+c to cancel)")? {
                    if let Ok(count) = input.parse() {
                        let result = s.version_control.log(count);
                        s.handle_result(h, result)
                    } else {
                        s.show_header(h, HeaderKind::Error)?;
                        queue!(
                            s.write,
                            Print("could not parse a number from "),
                            Print(input)
                        )?;
                        Ok(())
                    }
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['e'] => Ok(HandleChordResult::Unhandled),
            ['e', 'e'] => self.action_context("current full revision", |s, h| {
                let result = s.version_control.current_export();
                s.handle_result(h, result)
            }),
            ['d'] => Ok(HandleChordResult::Unhandled),
            ['d', 'd'] => self.action_context("current diff all", |s, h| {
                let result = s.version_control.current_diff_all();
                s.handle_result(h, result)
            }),
            ['d', 's'] => self.action_context("current diff selected", |s, h| {
                match s.version_control.get_current_changed_files() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            let result = s.version_control.current_diff_selected(&entries);
                            s.handle_result(h, result)
                        } else {
                            s.show_header(h, HeaderKind::Canceled)
                        }
                    }
                    Err(error) => s.handle_result(h, Err(error)),
                }
            }),
            ['D'] => Ok(HandleChordResult::Unhandled),
            ['D', 'C'] => self.action_context("revision changes", |s, h| {
                if let Some(input) = s.handle_input("show changes from (ctrl+c to cancel): ")? {
                    let result = s.version_control.revision_changes(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['D', 'D'] => self.action_context("revision diff all", |s, h| {
                if let Some(input) = s.handle_input("show diff from (ctrl+c to cancel): ")? {
                    let result = s.version_control.revision_diff_all(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['D', 'S'] => self.action_context("revision diff selected", |s, h| {
                if let Some(input) = s.handle_input("show diff from (ctrl+c to cancel): ")? {
                    match s.version_control.get_revision_changed_files(&input[..]) {
                        Ok(mut entries) => {
                            if s.show_select_ui(&mut entries)? {
                                let result = s.version_control.revision_diff_selected(&input[..], &entries);
                                s.handle_result(h, result)
                            } else {
                                s.show_header(h, HeaderKind::Canceled)
                            }
                        }
                        Err(error) => s.handle_result(h, Err(error)),
                    }
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['c'] => Ok(HandleChordResult::Unhandled),
            ['c', 'c'] => self.action_context("commit all", |s, h| {
                if let Some(input) = s.handle_input("commit message (ctrl+c to cancel): ")? {
                    let result = s.version_control.commit_all(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['c', 's'] => self.action_context("commit selected", |s, h| {
                match s.version_control.get_current_changed_files() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            s.show_header(h, HeaderKind::Waiting)?;
                            if let Some(input) =
                                s.handle_input("commit message (ctrl+c to cancel): ")?
                            {
                                let result =
                                    s.version_control.commit_selected(&input[..], &entries);
                                s.handle_result(h, result)
                            } else {
                                s.show_header(h, HeaderKind::Canceled)
                            }
                        } else {
                            s.show_header(h, HeaderKind::Canceled)
                        }
                    }
                    Err(error) => s.handle_result(h, Err(error)),
                }
            }),
            ['u'] => self.action_context("update", |s, h| {
                if let Some(input) = s.handle_input("update to (ctrl+c to cancel): ")? {
                    let result = s.version_control.update(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['m'] => self.action_context("merge", |s, h| {
                if let Some(input) = s.handle_input("merge with (ctrl+c to cancel): ")? {
                    let result = s.version_control.merge(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['R'] => Ok(HandleChordResult::Unhandled),
            ['R', 'A'] => self.action_context("revert all", |s, h| {
                let result = s.version_control.revert_all();
                s.handle_result(h, result)
            }),
            ['r'] => Ok(HandleChordResult::Unhandled),
            ['r', 's'] => self.action_context("revert selected", |s, h| {
                match s.version_control.get_current_changed_files() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            let result = s.version_control.revert_selected(&entries);
                            s.handle_result(h, result)
                        } else {
                            s.show_header(h, HeaderKind::Canceled)
                        }
                    }
                    Err(error) => s.handle_result(h, Err(error)),
                }
            }),
            ['r', 'r'] => self.action_context("unresolved conflicts", |s, h| {
                let result = s.version_control.conflicts();
                s.handle_result(h, result)
            }),
            ['r', 'o'] => self.action_context("merge taking other", |s, h| {
                let result = s.version_control.take_other();
                s.handle_result(h, result)
            }),
            ['r', 'l'] => self.action_context("merge taking local", |s, h| {
                let result = s.version_control.take_local();
                s.handle_result(h, result)
            }),
            ['f'] => self.action_context("fetch", |s, h| {
                let result = s.version_control.fetch();
                s.handle_result(h, result)
            }),
            ['p'] => self.action_context("pull", |s, h| {
                let result = s.version_control.pull();
                s.handle_result(h, result)
            }),
            ['P'] => self.action_context("push", |s, h| {
                let result = s.version_control.push();
                s.handle_result(h, result)
            }),
            ['t'] => Ok(HandleChordResult::Unhandled),
            ['t', 'n'] => self.action_context("new tag", |s, h| {
                if let Some(input) = s.handle_input("new tag name (ctrl+c to cancel): ")? {
                    let result = s.version_control.create_tag(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['b'] => Ok(HandleChordResult::Unhandled),
            ['b', 'b'] => self.action_context("list branches", |s, h| {
                let result = s.version_control.list_branches();
                s.handle_result(h, result)
            }),
            ['b', 'n'] => self.action_context("new branch", |s, h| {
                if let Some(input) = s.handle_input("new branch name (ctrl+c to cancel): ")? {
                    let result = s.version_control.create_branch(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['b', 'd'] => self.action_context("delete branch", |s, h| {
                if let Some(input) = s.handle_input("branch to delete (ctrl+c to cancel): ")? {
                    let result = s.version_control.close_branch(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['x'] => self.action_context("custom action", |s, h| {
                if s.custom_actions.len() > 0 {
                    for c in &s.custom_actions {
                        s.write
                            .queue(SetForegroundColor(ENTRY_COLOR))?
                            .queue(Print(&c.shortcut))?
                            .queue(ResetColor)?
                            .queue(Print('\t'))?
                            .queue(Print(&c.command))?;
                        for a in &c.args {
                            s.write.queue(Print(' '))?.queue(Print(a))?;
                        }
                        s.write.queue(cursor::MoveToNextLine(1))?;
                    }
                    s.handle_custom_action(h)?;
                    s.current_key_chord.clear();
                } else {
                    s.show_header(h, HeaderKind::Error)?;
                    queue!(
                        s.write,
                        ResetColor,
                        Print("no commands available"),
                        cursor::MoveToNextLine(2),
                        Print("create custom actions by placing them inside '.verco/custom_actions.txt'"),
                    )?;
                }
                Ok(())
            }),
            _ => Ok(HandleChordResult::Handled),
        }
    }

    fn handle_custom_action(&mut self, header: &Header) -> Result<()> {
        self.current_key_chord.clear();
        self.write.queue(cursor::SavePosition)?;

        'outer: loop {
            self.write.flush()?;
            match input::read_key()? {
                KeyEvent {
                    code: KeyCode::Esc, ..
                }
                | KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    return self.show_header(header, HeaderKind::Canceled);
                }
                key_event => {
                    if let Some(c) = input::key_to_char(key_event) {
                        self.current_key_chord.push(c);
                    }
                    for action in &self.custom_actions {
                        if action
                            .shortcut
                            .chars()
                            .zip(
                                self.current_key_chord
                                    .iter()
                                    .map(|c| *c)
                                    .chain(iter::repeat('\0')),
                            )
                            .all(|(a, b)| a == b)
                        {
                            self.write
                                .queue(cursor::RestorePosition)?
                                .queue(cursor::MoveToNextLine(2))?
                                .queue(SetForegroundColor(ENTRY_COLOR))?
                                .queue(Print(&action.command))?
                                .queue(ResetColor)?;
                            for arg in &action.args {
                                self.write
                                    .queue(Print(' '))?
                                    .queue(Print(arg))?;
                            }
                            self.write.queue(cursor::MoveToNextLine(2))?;

                            let result =
                                action.execute(self.version_control.get_root());
                            self.handle_result(header, result)?;
                            return Ok(());
                        }
                    }
                    self.show_current_key_chord()?;

                    for action in &self.custom_actions {
                        if action
                            .shortcut
                            .chars()
                            .zip(&self.current_key_chord)
                            .all(|(a, b)| a == *b)
                        {
                            continue 'outer;
                        }
                    }

                    self.show_header(header, HeaderKind::Canceled)?;
                    self.write.queue(Print("no match found"))?;
                    return Ok(());
                }
            }
        }
    }

    fn handle_input(&mut self, prompt: &str) -> Result<Option<String>> {
        execute!(
            self.write,
            SetForegroundColor(ENTRY_COLOR),
            Print(prompt),
            ResetColor,
            cursor::MoveToNextLine(1),
            cursor::Show,
        )?;

        let res = match input::read_line() {
            Ok(line) => {
                if line.len() > 0 {
                    Some(line)
                } else {
                    None
                }
            }
            Err(_error) => None,
        };
        self.write.execute(cursor::Hide)?;
        Ok(res)
    }

    fn handle_result(
        &mut self,
        header: &Header,
        result: std::result::Result<String, String>,
    ) -> Result<()> {
        let output = match result {
            Ok(output) => {
                show_header(&mut self.write, header, HeaderKind::Ok)?;
                output
            }
            Err(error) => {
                show_header(&mut self.write, header, HeaderKind::Error)?;
                error
            }
        };

        self.scroll_view.set_content(output);
        self.scroll_view.show(&mut self.write)
    }

    fn show_current_key_chord(&mut self) -> Result<()> {
        let (w, h) = terminal::size()?;
        queue!(
            self.write,
            cursor::MoveTo(w - self.current_key_chord.len() as u16, h - 1),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ENTRY_COLOR),
        )?;
        for c in &self.current_key_chord {
            self.write.queue(Print(c))?;
        }
        self.write.queue(ResetColor)?;
        Ok(())
    }

    fn show_help(
        &mut self,
        header: &Header,
    ) -> Result<std::result::Result<String, String>> {
        let mut write = Vec::with_capacity(1024);

        queue!(
            &mut write,
            Print("Verco "),
            Print(VERSION),
            cursor::MoveToNextLine(2),
        )?;

        match self.version_control.version() {
            Ok(version) => {
                queue!(&mut write, Print(version), cursor::MoveToNextLine(2))?;
            }
            Err(error) => {
                self.show_header(header, HeaderKind::Error)?;
                write.queue(Print(error))?;
                write.flush()?;
                return Ok(Err(String::from_utf8(write)?));
            }
        }

        write
            .queue(Print("press a key and peform an action"))?
            .queue(cursor::MoveToNextLine(2))?;

        Self::show_help_action(&mut write, "h", "help")?;
        Self::show_help_action(&mut write, "q", "quit")?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "s", "status")?;
        Self::show_help_action(&mut write, "ll", "log")?;
        Self::show_help_action(&mut write, "lc", "log count")?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "ee", "revision full contents")?;
        Self::show_help_action(&mut write, "dd", "current diff all")?;
        Self::show_help_action(&mut write, "ds", "current diff selected")?;
        Self::show_help_action(&mut write, "DC", "revision changes")?;
        Self::show_help_action(&mut write, "DD", "revision diff all")?;
        Self::show_help_action(&mut write, "DS", "revision diff selected")?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "cc", "commit all")?;
        Self::show_help_action(&mut write, "cs", "commit selected")?;
        Self::show_help_action(&mut write, "u", "update/checkout")?;
        Self::show_help_action(&mut write, "m", "merge")?;
        Self::show_help_action(&mut write, "RA", "revert all")?;
        Self::show_help_action(&mut write, "rs", "revert selected")?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "rr", "list unresolved conflicts")?;
        Self::show_help_action(&mut write, "ro", "resolve taking other")?;
        Self::show_help_action(&mut write, "rl", "resolve taking local")?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "f", "fetch")?;
        Self::show_help_action(&mut write, "p", "pull")?;
        Self::show_help_action(&mut write, "P", "push")?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "tn", "new tag")?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "bb", "list branches")?;
        Self::show_help_action(&mut write, "bn", "new branch")?;
        Self::show_help_action(&mut write, "bd", "delete branch")?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "x", "custom action")?;

        write.flush()?;
        Ok(Ok(String::from_utf8(write)?))
    }

    fn show_help_action<HW>(
        write: &mut HW,
        shortcut: &str,
        action: &str,
    ) -> Result<()>
    where
        HW: Write,
    {
        queue!(
            write,
            SetForegroundColor(ENTRY_COLOR),
            Print('\t'),
            Print(shortcut),
            ResetColor,
            Print('\t'),
            Print('\t'),
            Print(action),
            cursor::MoveToNextLine(1),
        )
    }
}
