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
    application::{Action, ActionFuture, ActionResult, Application},
    input,
    scroll_view::ScrollView,
    select::{select, Entry},
    tui_util::{show_header, Header, HeaderKind, ENTRY_COLOR},
    worker::Task,
};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn show_tui(application: Application) {
    Tui::new(application, stdout().lock()).show().unwrap();
}

enum HandleChordResult {
    Handled,
    Unhandled,
    Quit,
}

struct Tui<W>
where
    W: Write,
{
    application: Application,
    current_action: Action,

    write: W,
    scroll_view: ScrollView,
}

impl<W> Tui<W>
where
    W: Write,
{
    fn new(application: Application, write: W) -> Self {
        Tui {
            application,
            current_action: Action::Quit,
            write,
            scroll_view: Default::default(),
        }
    }

    fn show_header(&self, kind: HeaderKind) -> Result<()> {
        let header = Header {
            action_name: self.current_action.name(),
            directory_name: self.application.version_control.get_root(),
        };
        show_header(&mut self.write, header, kind)
    }

    fn show_select_ui(&mut self, entries: &mut Vec<Entry>) -> Result<bool> {
        self.show_header(HeaderKind::Waiting)?;
        select(&mut self.write, entries)
    }

    fn handle_action(
        &mut self,
        task: Box<dyn Task<Output = ActionResult>>,
    ) -> Result<()> {
        let result = self.application.run_action(ActionFuture {
            action: self.current_action,
            task,
        });
        self.handle_result(&result.0)
    }

    fn action_context<F>(
        &mut self,
        action: Action,
        callback: F,
    ) -> Result<HandleChordResult>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        self.current_action = action;
        callback(self).map(|_| HandleChordResult::Handled)
    }

    fn show(&mut self) -> Result<()> {
        execute!(self.write, EnterAlternateScreen, cursor::Hide)?;
        terminal::enable_raw_mode()?;

        self.current_action = Action::Help;
        self.handle_result(&self.show_help()?)?;

        let (w, h) = terminal::size()?;
        queue!(
            self.write,
            cursor::MoveTo(w, h - 1),
            Clear(ClearType::CurrentLine),
        )?;

        loop {
            if let Some((action, result)) =
                self.application.poll_action_result()
            {
                if self.current_action == action {
                    self.handle_result(&result.0)?;
                }
            }

            self.write.flush()?;
            match input::read_key()? {
                KeyEvent {
                    code: KeyCode::Esc, ..
                }
                | KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    if self.application.current_key_chord.len() == 0 {
                        break;
                    }

                    self.application.current_key_chord.clear();
                    self.show_current_key_chord()?;
                }
                key_event => {
                    if self.scroll_view.update(&mut self.write, &key_event)? {
                        continue;
                    }

                    if let Some(c) = input::key_to_char(key_event) {
                        self.application.current_key_chord.push(c);
                    }
                    match self.handle_key_chord()? {
                        HandleChordResult::Handled => {
                            self.application.current_key_chord.clear()
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

    fn handle_key_chord(&mut self) -> Result<HandleChordResult> {
        match &self.application.current_key_chord[..] {
            ['q'] => Ok(HandleChordResult::Quit),
            ['h'] => {
            self.current_action = Action::Help;
            self.handle_result(&self.show_help()?)?;
                Ok(HandleChordResult::Handled)
            }
            ['s'] => self.action_context(Action::Status, |s| {
                s.handle_action(s.application.version_control.status())
            }),
            ['l'] => Ok(HandleChordResult::Unhandled),
            ['l', 'l'] => self.action_context(Action::Log, |s| {
                let (_w, h) = terminal::size()?;
                s.handle_action(s.application.version_control.log(h as usize))
            }),
            ['l', 'c'] => self.action_context(Action::LogCount, |s| {
                if let Some(input) = s.handle_input("logs to show (ctrl+c to cancel)")? {
                    if let Ok(count) = input.parse() {
                        s.handle_action(s.application.version_control.log(count))
                    } else {
                        s.show_header(HeaderKind::Error)?;
                        queue!(
                            s.write,
                            Print("could not parse a number from "),
                            Print(input)
                        )
                    }
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['e'] => Ok(HandleChordResult::Unhandled),
            ['e', 'e'] => self.action_context(Action::CurrentFullRevision, |s| {
                s.handle_action( s.application.version_control.current_export())
            }),
            ['d'] => Ok(HandleChordResult::Unhandled),
            ['d', 'd'] => self.action_context(Action::CurrentDiffAll, |s| {
                s.handle_action( s.application.version_control.current_diff_all())
            }),
            ['d', 's'] => self.action_context(Action::CurrentDiffSelected, |s| {
                match s.application.version_control.get_current_changed_files() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            s.handle_action( s.application.version_control.current_diff_selected(&entries))
                        } else {
                            s.show_header( HeaderKind::Canceled)
                        }
                    }
                    Err(error) => s.handle_result( &Err(error)),
                }
            }),
            ['D'] => Ok(HandleChordResult::Unhandled),
            ['D', 'C'] => self.action_context(Action::RevisionChanges, |s| {
                if let Some(input) = s.handle_input("show changes from (ctrl+c to cancel)")? {
                    s.handle_action( s.application.version_control.revision_changes(&input[..]))
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['D', 'D'] => self.action_context(Action::RevisionDiffAll, |s| {
                if let Some(input) = s.handle_input("show diff from (ctrl+c to cancel)")? {
                    s.handle_action( s.application.version_control.revision_diff_all(&input[..]))
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['D', 'S'] => self.action_context(Action::RevisionDiffSelected, |s| {
                if let Some(input) = s.handle_input("show diff from (ctrl+c to cancel)")? {
                    match s.application.version_control.get_revision_changed_files(&input[..]) {
                        Ok(mut entries) => {
                            if s.show_select_ui(&mut entries)? {
                                s.handle_action( s.application.version_control.revision_diff_selected(&input[..], &entries))
                            } else {
                                s.show_header( HeaderKind::Canceled)
                            }
                        }
                        Err(error) => s.handle_result( &Err(error)),
                    }
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['c'] => Ok(HandleChordResult::Unhandled),
            ['c', 'c'] => self.action_context(Action::CommitAll, |s| {
                if let Some(input) = s.handle_input("commit message (ctrl+c to cancel)")? {
                    s.handle_action( s.application.version_control.commit_all(&input[..]))
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['c', 's'] => self.action_context(Action::CommitSelected, |s| {
                match s.application.version_control.get_current_changed_files() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            s.show_header( HeaderKind::Waiting)?;
                            if let Some(input) =
                                s.handle_input("commit message (ctrl+c to cancel)")?
                            {
                                s.handle_action(
                                    s.application.version_control.commit_selected(&input[..], &entries))
                            } else {
                                s.show_header( HeaderKind::Canceled)
                            }
                        } else {
                            s.show_header( HeaderKind::Canceled)
                        }
                    }
                    Err(error) => s.handle_result( &Err(error)),
                }
            }),
            ['u'] => self.action_context(Action::Update, |s| {
                if let Some(input) = s.handle_input("update to (ctrl+c to cancel)")? {
                    s.handle_action( s.application.version_control.update(&input[..]))
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['m'] => self.action_context(Action::Merge, |s| {
                if let Some(input) = s.handle_input("merge with (ctrl+c to cancel)")? {
                    s.handle_action( s.application.version_control.merge(&input[..]))
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['R'] => Ok(HandleChordResult::Unhandled),
            ['R', 'A'] => self.action_context(Action::RevertAll, |s| {
                s.handle_action( s.application.version_control.revert_all())
            }),
            ['r'] => Ok(HandleChordResult::Unhandled),
            ['r', 's'] => self.action_context(Action::RevertSelected, |s| {
                match s.application.version_control.get_current_changed_files() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            s.handle_action( s.application.version_control.revert_selected(&entries))
                        } else {
                            s.show_header( HeaderKind::Canceled)
                        }
                    }
                    Err(error) => s.handle_result( &Err(error)),
                }
            }),
            ['r', 'r'] => self.action_context(Action::UnresolvedConflicts, |s| {
                s.handle_action( s.application.version_control.conflicts())
            }),
            ['r', 'o'] => self.action_context(Action::MergeTakingOther, |s| {
                s.handle_action( s.application.version_control.take_other())
            }),
            ['r', 'l'] => self.action_context(Action::MergeTakingLocal, |s| {
                s.handle_action( s.application.version_control.take_local())
            }),
            ['f'] => self.action_context(Action::Fetch, |s| {
                s.handle_action( s.application.version_control.fetch())
            }),
            ['p'] => self.action_context(Action::Pull, |s| {
                s.handle_action( s.application.version_control.pull())
            }),
            ['P'] => self.action_context(Action::Push, |s| {
                s.handle_action( s.application.version_control.push())
            }),
            ['t'] => Ok(HandleChordResult::Unhandled),
            ['t', 'n'] => self.action_context(Action::NewTag, |s| {
                if let Some(input) = s.handle_input("new tag name (ctrl+c to cancel)")? {
                    s.handle_action( s.application.version_control.create_tag(&input[..]))
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['b'] => Ok(HandleChordResult::Unhandled),
            ['b', 'b'] => self.action_context(Action::ListBranches, |s| {
                s.handle_action( s.application.version_control.list_branches())
            }),
            ['b', 'n'] => self.action_context(Action::NewBranch, |s| {
                if let Some(input) = s.handle_input("new branch name (ctrl+c to cancel)")? {
                    s.handle_action( s.application.version_control.create_branch(&input[..]))
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['b', 'd'] => self.action_context(Action::DeleteBranch, |s| {
                if let Some(input) = s.handle_input("branch to delete (ctrl+c to cancel)")? {
                    s.handle_action( s.application.version_control.close_branch(&input[..]))
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['x'] => self.action_context(Action::CustomAction, |s| {
                if s.application.custom_actions.len() > 0 {
                    for c in &s.application.custom_actions {
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
                    s.handle_custom_action()?;
                    s.application.current_key_chord.clear();
                } else {
                    s.show_header( HeaderKind::Error)?;
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

    fn handle_custom_action(&mut self) -> Result<()> {
        self.application.current_key_chord.clear();
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
                    return self.show_header(HeaderKind::Canceled);
                }
                key_event => {
                    if let Some(c) = input::key_to_char(key_event) {
                        self.application.current_key_chord.push(c);
                    }
                    for action in &self.application.custom_actions {
                        if action
                            .shortcut
                            .chars()
                            .zip(
                                self.application
                                    .current_key_chord
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

                            let result = action.execute(
                                self.application.version_control.get_root(),
                            );
                            self.handle_result(&result)?;
                            return Ok(());
                        }
                    }
                    self.show_current_key_chord()?;

                    for action in &self.application.custom_actions {
                        if action
                            .shortcut
                            .chars()
                            .zip(&self.application.current_key_chord)
                            .all(|(a, b)| a == *b)
                        {
                            continue 'outer;
                        }
                    }

                    self.show_header(HeaderKind::Canceled)?;
                    self.write.queue(Print("no match found"))?;
                    return Ok(());
                }
            }
        }
    }

    fn handle_input(&mut self, prompt: &str) -> Result<Option<String>> {
        self.show_header(HeaderKind::Waiting)?;
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
        result: &std::result::Result<String, String>,
    ) -> Result<()> {
        let output = match result {
            Ok(output) => {
                self.show_header(HeaderKind::Ok)?;
                output
            }
            Err(error) => {
                self.show_header(HeaderKind::Error)?;
                error
            }
        };

        self.scroll_view.set_content(&output[..]);
        self.scroll_view.show(&mut self.write)
    }

    fn show_current_key_chord(&mut self) -> Result<()> {
        let (w, h) = terminal::size()?;
        queue!(
            self.write,
            cursor::MoveTo(
                w - self.application.current_key_chord.len() as u16,
                h - 1
            ),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ENTRY_COLOR),
        )?;
        for c in &self.application.current_key_chord {
            self.write.queue(Print(c))?;
        }
        self.write.queue(ResetColor)?;
        Ok(())
    }

    fn show_help(&mut self) -> Result<std::result::Result<String, String>> {
        let mut write = Vec::with_capacity(1024);

        queue!(
            &mut write,
            Print("Verco "),
            Print(VERSION),
            cursor::MoveToNextLine(2),
        )?;

        if let Ok(version) = self.application.version_control.version() {
            queue!(&mut write, Print(version), cursor::MoveToNextLine(2))?;
        }

        write
            .queue(Print("press a key and peform an action"))?
            .queue(cursor::MoveToNextLine(2))?;

        Self::show_help_action(&mut write, "h", Action::Help)?;
        Self::show_help_action(&mut write, "q", Action::Quit)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "s", Action::Status)?;
        Self::show_help_action(&mut write, "ll", Action::Log)?;
        Self::show_help_action(&mut write, "lc", Action::LogCount)?;

        Self::show_help_action(&mut write, "ee", Action::CurrentFullRevision)?;
        Self::show_help_action(&mut write, "dd", Action::CurrentDiffAll)?;
        Self::show_help_action(&mut write, "ds", Action::CurrentDiffSelected)?;
        Self::show_help_action(&mut write, "DC", Action::RevisionChanges)?;
        Self::show_help_action(&mut write, "DD", Action::RevisionDiffAll)?;
        Self::show_help_action(&mut write, "DS", Action::RevisionDiffSelected)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "cc", Action::CommitAll)?;
        Self::show_help_action(&mut write, "cs", Action::CommitSelected)?;
        Self::show_help_action(&mut write, "u", Action::Update)?;
        Self::show_help_action(&mut write, "m", Action::Merge)?;
        Self::show_help_action(&mut write, "RA", Action::RevertAll)?;
        Self::show_help_action(&mut write, "rs", Action::RevertSelected)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "rr", Action::UnresolvedConflicts)?;
        Self::show_help_action(&mut write, "ro", Action::MergeTakingOther)?;
        Self::show_help_action(&mut write, "rl", Action::MergeTakingLocal)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "f", Action::Fetch)?;
        Self::show_help_action(&mut write, "p", Action::Pull)?;
        Self::show_help_action(&mut write, "P", Action::Push)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "tn", Action::NewTag)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "bb", Action::ListBranches)?;
        Self::show_help_action(&mut write, "bn", Action::NewBranch)?;
        Self::show_help_action(&mut write, "bd", Action::DeleteBranch)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "x", Action::CustomAction)?;

        write.flush()?;
        Ok(Ok(String::from_utf8(write)?))
    }

    fn show_help_action<HW>(
        write: &mut HW,
        shortcut: &str,
        action: Action,
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
            Print(action.name()),
            cursor::MoveToNextLine(1),
        )
    }
}
