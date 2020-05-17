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
    iter, thread,
    time::Duration,
};

use crate::{
    action::{ActionKind, ActionResult, ActionTask},
    application::{ActionFuture, Application},
    input::{self, Event},
    scroll_view::ScrollView,
    select::{select, Entry},
    tui_util::{show_header, Header, HeaderKind, ENTRY_COLOR},
};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn show_tui(application: Application) {
    let stdout = stdout();
    let stdout = stdout.lock();
    let mut tui = Tui::new(application, stdout);
    tui.show().unwrap();
    tui.application.stop();
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
    current_action_kind: ActionKind,

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
            current_action_kind: ActionKind::Quit,
            write,
            scroll_view: Default::default(),
        }
    }

    fn show_header(&mut self, kind: HeaderKind) -> Result<()> {
        let header = Header {
            action_name: self.current_action_kind.name(),
            directory_name: self.application.version_control.get_root(),
        };
        show_header(&mut self.write, header, kind)
    }

    fn show_select_ui(&mut self, entries: &mut Vec<Entry>) -> Result<bool> {
        self.show_header(HeaderKind::Waiting)?;
        select(&mut self.write, entries)
    }

    fn show_action(&mut self, task: Box<dyn ActionTask>) -> Result<()> {
        let result = self.application.run_action(ActionFuture {
            kind: self.current_action_kind,
            task,
        });
        let output = match result {
            ActionResult::Ok(output) => output,
            ActionResult::Err(error) => error,
        };
        self.show_header(HeaderKind::Waiting)?;
        self.scroll_view.set_content(&output[..]);
        self.scroll_view.show(&mut self.write)
    }

    fn action_context<F>(
        &mut self,
        action: ActionKind,
        callback: F,
    ) -> Result<HandleChordResult>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        self.current_action_kind = action;
        callback(self).map(|_| HandleChordResult::Handled)
    }

    fn show(&mut self) -> Result<()> {
        execute!(self.write, EnterAlternateScreen, cursor::Hide)?;
        terminal::enable_raw_mode()?;

        {
            self.current_action_kind = ActionKind::Help;
            let help = self.show_help()?;
            self.show_result(&help)?;
        }

        let (w, h) = terminal::size()?;
        queue!(
            self.write,
            cursor::MoveTo(w, h - 1),
            Clear(ClearType::CurrentLine),
        )?;

        loop {
            if let Some(result) = self
                .application
                .poll_and_check_action(self.current_action_kind)
            {
                self.show_result(&result)?;
                self.write.flush()?;
            }

            match input::poll_event() {
                Event::Resize => {}
                Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                }) => {
                    if self.application.current_key_chord.len() == 0 {
                        break;
                    }

                    self.application.current_key_chord.clear();
                    self.show_current_key_chord()?;
                    self.write.flush()?;
                }
                Event::Key(key_event) => {
                    if self.scroll_view.update(&mut self.write, &key_event)? {
                        self.write.flush()?;
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
                    self.write.flush()?;
                }
                _ => (),
            }

            thread::sleep(Duration::from_millis(20));
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
            self.current_action_kind = ActionKind::Help;
            let help = self.show_help()?;
            self.show_result(&help)?;
                Ok(HandleChordResult::Handled)
            }
            ['s'] => self.action_context(ActionKind::Status, |s| {
                let action =s.application.version_control.status();
                s.show_action(action)
            }),
            ['l'] => Ok(HandleChordResult::Unhandled),
            ['l', 'l'] => self.action_context(ActionKind::Log, |s| {
                let (_w, h) = terminal::size()?;
                let action = s.application.version_control.log(h as usize);
                s.show_action(action)
            }),
            ['l', 'c'] => self.action_context(ActionKind::LogCount, |s| {
                if let Some(input) = s.handle_input("logs to show (ctrl+c to cancel)")? {
                    if let Ok(count) = input.trim().parse() {
                        let action = s.application.version_control.log(count);
                        s.show_action(action)
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
            ['e', 'e'] => self.action_context(ActionKind::CurrentFullRevision, |s| {
                let action =  s.application.version_control.current_export();
                s.show_action(action)
            }),
            ['d'] => Ok(HandleChordResult::Unhandled),
            ['d', 'd'] => self.action_context(ActionKind::CurrentDiffAll, |s| {
                let action =  s.application.version_control.current_diff_all();
                s.show_action(action)
            }),
            ['d', 's'] => self.action_context(ActionKind::CurrentDiffSelected, |s| {
                match s.application.version_control.get_current_changed_files() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            let action =  s.application.version_control.current_diff_selected(&entries);
                            s.show_action(action)
                        } else {
                            s.show_header( HeaderKind::Canceled)
                        }
                    }
                    Err(error) => s.show_result( &ActionResult::Err(error)),
                }
            }),
            ['D'] => Ok(HandleChordResult::Unhandled),
            ['D', 'C'] => self.action_context(ActionKind::RevisionChanges, |s| {
                if let Some(input) = s.handle_input("show changes from (ctrl+c to cancel)")? {
                    let action =  s.application.version_control.revision_changes(&input[..]);
                    s.show_action(action)
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['D', 'D'] => self.action_context(ActionKind::RevisionDiffAll, |s| {
                if let Some(input) = s.handle_input("show diff from (ctrl+c to cancel)")? {
                    let action =  s.application.version_control.revision_diff_all(&input[..]);
                    s.show_action(action)
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['D', 'S'] => self.action_context(ActionKind::RevisionDiffSelected, |s| {
                if let Some(input) = s.handle_input("show diff from (ctrl+c to cancel)")? {
                    match s.application.version_control.get_revision_changed_files(&input[..]) {
                        Ok(mut entries) => {
                            if s.show_select_ui(&mut entries)? {
                                let action =  s.application.version_control.revision_diff_selected(&input[..], &entries);
                                s.show_action(action)
                            } else {
                                s.show_header( HeaderKind::Canceled)
                            }
                        }
                        Err(error) => s.show_result( &ActionResult::Err(error)),
                    }
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['c'] => Ok(HandleChordResult::Unhandled),
            ['c', 'c'] => self.action_context(ActionKind::CommitAll, |s| {
                if let Some(input) = s.handle_input("commit message (ctrl+c to cancel)")? {
                    let action =  s.application.version_control.commit_all(&input[..]);
                    s.show_action(action)
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['c', 's'] => self.action_context(ActionKind::CommitSelected, |s| {
                match s.application.version_control.get_current_changed_files() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            s.show_header( HeaderKind::Waiting)?;
                            if let Some(input) =
                                s.handle_input("commit message (ctrl+c to cancel)")?
                            {
                                let action =  s.application.version_control.commit_selected(&input[..], &entries);
                                s.show_action(action)
                            } else {
                                s.show_header( HeaderKind::Canceled)
                            }
                        } else {
                            s.show_header( HeaderKind::Canceled)
                        }
                    }
                    Err(error) => s.show_result( &ActionResult::Err(error)),
                }
            }),
            ['u'] => self.action_context(ActionKind::Update, |s| {
                if let Some(input) = s.handle_input("update to (ctrl+c to cancel)")? {
                    let action =  s.application.version_control.update(&input[..]);
                    s.show_action(action)
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['m'] => self.action_context(ActionKind::Merge, |s| {
                if let Some(input) = s.handle_input("merge with (ctrl+c to cancel)")? {
                    let action =  s.application.version_control.merge(&input[..]);
                    s.show_action(action)
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['R'] => Ok(HandleChordResult::Unhandled),
            ['R', 'A'] => self.action_context(ActionKind::RevertAll, |s| {
                let action =  s.application.version_control.revert_all();
                s.show_action(action)
            }),
            ['r'] => Ok(HandleChordResult::Unhandled),
            ['r', 's'] => self.action_context(ActionKind::RevertSelected, |s| {
                match s.application.version_control.get_current_changed_files() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            let action =  s.application.version_control.revert_selected(&entries);
                            s.show_action(action)
                        } else {
                            s.show_header( HeaderKind::Canceled)
                        }
                    }
                    Err(error) => s.show_result( &ActionResult::Err(error)),
                }
            }),
            ['r', 'r'] => self.action_context(ActionKind::UnresolvedConflicts, |s| {
                let action =  s.application.version_control.conflicts();
                s.show_action(action)
            }),
            ['r', 'o'] => self.action_context(ActionKind::MergeTakingOther, |s| {
                let action =  s.application.version_control.take_other();
                s.show_action(action)
            }),
            ['r', 'l'] => self.action_context(ActionKind::MergeTakingLocal, |s| {
                let action =  s.application.version_control.take_local();
                s.show_action(action)
            }),
            ['f'] => self.action_context(ActionKind::Fetch, |s| {
                let action =  s.application.version_control.fetch();
                s.show_action(action)
            }),
            ['p'] => self.action_context(ActionKind::Pull, |s| {
                let action =  s.application.version_control.pull();
                s.show_action(action)
            }),
            ['P'] => self.action_context(ActionKind::Push, |s| {
                let action =  s.application.version_control.push();
                s.show_action(action)
            }),
            ['t'] => Ok(HandleChordResult::Unhandled),
            ['t', 'n'] => self.action_context(ActionKind::NewTag, |s| {
                if let Some(input) = s.handle_input("new tag name (ctrl+c to cancel)")? {
                    let action =  s.application.version_control.create_tag(&input[..]);
                    s.show_action(action)
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['b'] => Ok(HandleChordResult::Unhandled),
            ['b', 'b'] => self.action_context(ActionKind::ListBranches, |s| {
                let action =  s.application.version_control.list_branches();
                s.show_action(action)
            }),
            ['b', 'n'] => self.action_context(ActionKind::NewBranch, |s| {
                if let Some(input) = s.handle_input("new branch name (ctrl+c to cancel)")? {
                    let action =  s.application.version_control.create_branch(&input[..]);
                    s.show_action(action)
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['b', 'd'] => self.action_context(ActionKind::DeleteBranch, |s| {
                if let Some(input) = s.handle_input("branch to delete (ctrl+c to cancel)")? {
                    let action =  s.application.version_control.close_branch(&input[..]);
                    s.show_action(action)
                } else {
                    s.show_header( HeaderKind::Canceled)
                }
            }),
            ['x'] => self.action_context(ActionKind::CustomAction, |s| {
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
            match input::poll_event() {
                Event::Resize => (),
                Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                }) => {
                    return self.show_header(HeaderKind::Canceled);
                }
                Event::Key(key_event) => {
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
                            self.show_result(&result)?;
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
                _ => (),
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

    fn show_result(&mut self, result: &ActionResult) -> Result<()> {
        let output = match result {
            ActionResult::Ok(output) => {
                self.show_header(HeaderKind::Ok)?;
                output
            }
            ActionResult::Err(error) => {
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

    fn show_help(&mut self) -> Result<ActionResult> {
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

        Self::show_help_action(&mut write, "h", ActionKind::Help)?;
        Self::show_help_action(&mut write, "q", ActionKind::Quit)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "s", ActionKind::Status)?;
        Self::show_help_action(&mut write, "ll", ActionKind::Log)?;
        Self::show_help_action(&mut write, "lc", ActionKind::LogCount)?;

        Self::show_help_action(
            &mut write,
            "ee",
            ActionKind::CurrentFullRevision,
        )?;
        Self::show_help_action(&mut write, "dd", ActionKind::CurrentDiffAll)?;
        Self::show_help_action(
            &mut write,
            "ds",
            ActionKind::CurrentDiffSelected,
        )?;
        Self::show_help_action(&mut write, "DC", ActionKind::RevisionChanges)?;
        Self::show_help_action(&mut write, "DD", ActionKind::RevisionDiffAll)?;
        Self::show_help_action(
            &mut write,
            "DS",
            ActionKind::RevisionDiffSelected,
        )?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "cc", ActionKind::CommitAll)?;
        Self::show_help_action(&mut write, "cs", ActionKind::CommitSelected)?;
        Self::show_help_action(&mut write, "u", ActionKind::Update)?;
        Self::show_help_action(&mut write, "m", ActionKind::Merge)?;
        Self::show_help_action(&mut write, "RA", ActionKind::RevertAll)?;
        Self::show_help_action(&mut write, "rs", ActionKind::RevertSelected)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(
            &mut write,
            "rr",
            ActionKind::UnresolvedConflicts,
        )?;
        Self::show_help_action(&mut write, "ro", ActionKind::MergeTakingOther)?;
        Self::show_help_action(&mut write, "rl", ActionKind::MergeTakingLocal)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "f", ActionKind::Fetch)?;
        Self::show_help_action(&mut write, "p", ActionKind::Pull)?;
        Self::show_help_action(&mut write, "P", ActionKind::Push)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "tn", ActionKind::NewTag)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "bb", ActionKind::ListBranches)?;
        Self::show_help_action(&mut write, "bn", ActionKind::NewBranch)?;
        Self::show_help_action(&mut write, "bd", ActionKind::DeleteBranch)?;

        write.queue(cursor::MoveToNextLine(1))?;

        Self::show_help_action(&mut write, "x", ActionKind::CustomAction)?;

        write.flush()?;
        Ok(ActionResult::Ok(String::from_utf8(write)?))
    }

    fn show_help_action<HW>(
        write: &mut HW,
        shortcut: &str,
        action: ActionKind,
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
