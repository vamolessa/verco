use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    ExecutableCommand, QueueableCommand, Result,
};

use std::{
    io::{stdout, Write},
    iter,
};

use crate::{
    ctrlc_handler::CtrlcHandler,
    custom_commands::CustomCommand,
    input,
    scroll_view::show_scroll_view,
    select::{select, Entry},
    tui_util::{show_header, Header, HeaderKind, ENTRY_COLOR},
    version_control_actions::VersionControlActions,
};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn show_tui(
    version_control: Box<dyn 'static + VersionControlActions>,
    custom_commands: Vec<CustomCommand>,
    ctrlc_handler: CtrlcHandler,
) {
    Tui::new(
        version_control,
        custom_commands,
        stdout().lock(),
        ctrlc_handler,
    )
    .show()
    .unwrap();
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
    version_control: Box<dyn 'static + VersionControlActions>,
    custom_commands: Vec<CustomCommand>,

    current_key_chord: Vec<char>,

    write: W,
    ctrlc_handler: CtrlcHandler,
}

impl<W> Tui<W>
where
    W: Write,
{
    fn new(
        version_control: Box<dyn 'static + VersionControlActions>,
        custom_commands: Vec<CustomCommand>,
        write: W,
        ctrlc_handler: CtrlcHandler,
    ) -> Self {
        Tui {
            version_control,
            custom_commands,
            current_key_chord: Vec::new(),
            write,
            ctrlc_handler,
        }
    }

    fn show_header(&mut self, header: &Header, kind: HeaderKind) -> Result<()> {
        show_header(&mut self.write, header, kind).map(|_| ())
    }

    fn show_select_ui(&mut self, entries: &mut Vec<Entry>) -> Result<bool> {
        select(&mut self.write, &mut self.ctrlc_handler, entries)
    }

    fn command_context<F>(&mut self, action_name: &str, callback: F) -> Result<HandleChordResult>
    where
        F: FnOnce(&mut Self, &Header) -> Result<()>,
    {
        let header = Header {
            action_name,
            directory_name: self.version_control.repository_directory().into(),
        };
        show_header(&mut self.write, &header, HeaderKind::Waiting)?;
        callback(self, &header).map(|_| HandleChordResult::Handled)
    }

    fn show(&mut self) -> Result<()> {
        queue!(self.write, cursor::Hide)?;
        self.command_context("help", |s, h| s.show_help(h))?;
        let (w, h) = terminal::size()?;
        queue!(
            self.write,
            cursor::MoveTo(w, h - 2),
            Clear(ClearType::CurrentLine),
        )?;

        loop {
            self.write.flush()?;
            match input::read_key(&mut self.ctrlc_handler)? {
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
                    let c = input::key_to_char(key_event);
                    self.current_key_chord.push(c);
                    match self.handle_command()? {
                        HandleChordResult::Handled => self.current_key_chord.clear(),
                        HandleChordResult::Unhandled => (),
                        HandleChordResult::Quit => break,
                    }
                    self.show_current_key_chord()?;
                }
            }
        }

        execute!(self.write, ResetColor, cursor::Show)?;
        Ok(())
    }

    fn handle_command(&mut self) -> Result<HandleChordResult> {
        match &self.current_key_chord[..] {
            ['q'] => Ok(HandleChordResult::Quit),
            ['h'] => self.command_context("help", |s, h| s.show_help(h)),
            ['s'] => self.command_context("status", |s, h| {
                let result =s.version_control.status();
                s.handle_result(h, result)
            }),
            ['l'] => Ok(HandleChordResult::Unhandled),
            ['l', 'l'] => self.command_context("log 20", |s, h| {
                let result = s.version_control.log(20);
                s.handle_result(h, result)
            }),
            ['l', 'c'] => self.command_context("log count", |s, h| {
                if let Some(input) = s.handle_input("logs to show (ctrl+c to cancel)")? {
                    if let Ok(count) = input.parse() {
                        let result = s.version_control.log(count);
                        s.handle_result(h, result)
                    } else {
                        s.show_header(h, HeaderKind::Error)?;
                        queue!(s.write, Print("could not parse a number from "), Print(input))?;
                        Ok(())
                    }
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['d'] => Ok(HandleChordResult::Unhandled),
            ['d', 'd'] => self.command_context("revision diff", |s, h| {
                if let Some(input) = s.handle_input("show diff from (ctrl+c to cancel): ")? {
                    let result = s.version_control.diff(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['d', 'c'] => self.command_context("revision changes", |s, h| {
                if let Some(input) = s.handle_input("show changes from (ctrl+c to cancel): ")? {
                    let result = s.version_control.changes(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['c'] => Ok(HandleChordResult::Unhandled),
            ['c', 'c'] => self.command_context("commit all", |s, h| {
                if let Some(input) = s.handle_input("commit message (ctrl+c to cancel): ")? {
                    let result = s.version_control.commit_all(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['c', 's'] => self.command_context("commit selected", |s, h| {
                match s.version_control.get_files_to_commit() {
                    Ok(mut entries) => {
                        if s.show_select_ui(&mut entries)? {
                            s.write.queue(Print('\n'))?;
                            if let Some(input) =
                                s.handle_input("commit message (ctrl+c to cancel): ")?
                            {
                                let result = s.version_control.commit_selected(&input[..], &entries);
                                s.handle_result(h,result)
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
            ['u'] => self.command_context("update", |s, h| {
                if let Some(input) = s.handle_input("update to (ctrl+c to cancel): ")? {
                    let result = s.version_control.update(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['m'] => self.command_context("merge", |s, h| {
                if let Some(input) = s.handle_input("merge with (ctrl+c to cancel): ")? {
                    let result = s.version_control.merge(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['R'] => Ok(HandleChordResult::Unhandled),
            ['R', 'a'] | ['R', 'A'] => self.command_context("revert all", |s, h| {
                let result = s.version_control.revert_all();
                s.handle_result(h, result)
            }),
            ['r'] => Ok(HandleChordResult::Unhandled),
            ['r', 's'] => self.command_context("revert selected", |s, h| {
                match s.version_control.get_files_to_commit() {
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
            ['r', 'r'] => self.command_context("unresolved conflicts", |s, h| {
                let result = s.version_control.conflicts();
                s.handle_result(h, result)
            }),
            ['r', 'o'] => self.command_context("merge taking other", |s, h| {
                let result =s.version_control.take_other();
                s.handle_result(h, result)
            }),
            ['r', 'l'] => self.command_context("merge taking local", |s, h| {
                let result = s.version_control.take_local();
                s.handle_result(h, result)
            }),
            ['f'] => self.command_context("fetch", |s, h| {
                let result = s.version_control.fetch();
                s.handle_result(h, result)
            }),
            ['p'] => self.command_context("pull", |s, h| {
                let result = s.version_control.pull();
                s.handle_result(h, result)
            }),
            ['P'] => self.command_context("push", |s, h| {
                let result = s.version_control.push();
                s.handle_result(h, result)
            }),
            ['t'] => Ok(HandleChordResult::Unhandled),
            ['t', 'n'] => self.command_context("new tag", |s, h| {
                if let Some(input) = s.handle_input("new tag name (ctrl+c to cancel): ")? {
                    let result = s.version_control.create_tag(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['b'] => Ok(HandleChordResult::Unhandled),
            ['b', 'b'] => self.command_context("list branches", |s, h| {
                let result = s.version_control.list_branches();
                s.handle_result(h, result)
            }),
            ['b', 'n'] => self.command_context("new branch", |s, h| {
                if let Some(input) = s.handle_input("new branch name (ctrl+c to cancel): ")? {
                    let result = s.version_control.create_branch(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['b', 'd'] => self.command_context("delete branch", |s, h| {
                if let Some(input) = s.handle_input("branch to delete (ctrl+c to cancel): ")? {
                    let result = s.version_control.close_branch(&input[..]);
                    s.handle_result(h, result)
                } else {
                    s.show_header(h, HeaderKind::Canceled)
                }
            }),
            ['x'] => self.command_context("custom command", |s, h| {
                if s.custom_commands.len() > 0 {
                    for c in &s.custom_commands {
                        s.write
                            .queue(SetForegroundColor(ENTRY_COLOR))?
                            .queue(Print(&c.shortcut))?
                            .queue(ResetColor)?
                            .queue(Print('\t'))?
                            .queue(Print(&c.command))?;
                        for a in &c.args {
                            s.write.queue(Print(' '))?.queue(Print(a))?;
                        }
                        s.write.queue(Print('\n'))?;
                    }
                    s.handle_custom_command(h)?;
                    s.current_key_chord.clear();
                } else {
                    s.show_header(h, HeaderKind::Error)?;
                    queue!(
                        s.write,
                        ResetColor,
                        Print("no commands available\n\ncreate custom commands by placing them inside './verco/custom_commands.txt'"),
                    )?;
                }
                Ok(())
            }),
            _ => Ok(HandleChordResult::Handled)
        }
    }

    fn handle_custom_command(&mut self, header: &Header) -> Result<()> {
        self.current_key_chord.clear();
        self.write.queue(cursor::SavePosition)?;

        'outer: loop {
            self.write.flush()?;
            match input::read_key(&mut self.ctrlc_handler)? {
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
                    let c = input::key_to_char(key_event);
                    self.current_key_chord.push(c);
                    for command in &self.custom_commands {
                        if command
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
                                .queue(Print('\n'))?
                                .queue(Print('\n'))?
                                .queue(SetForegroundColor(ENTRY_COLOR))?
                                .queue(Print(&command.command))?
                                .queue(ResetColor)?;
                            for arg in &command.args {
                                self.write.queue(Print(' '))?.queue(Print(arg))?;
                            }
                            self.write.queue(Print('\n'))?.queue(Print('\n'))?;

                            let result =
                                command.execute(self.version_control.repository_directory());
                            self.handle_result(header, result)?;
                            return Ok(());
                        }
                    }
                    self.show_current_key_chord()?;

                    for command in &self.custom_commands {
                        if command
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
            Print('\n'),
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
        self.ctrlc_handler.ignore_next();
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

        show_scroll_view(&mut self.write, &mut self.ctrlc_handler, &output[..])
    }

    fn show_current_key_chord(&mut self) -> Result<()> {
        let (w, h) = terminal::size()?;
        queue!(
            self.write,
            cursor::MoveTo(w - self.current_key_chord.len() as u16, h - 2),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ENTRY_COLOR),
        )?;
        for c in &self.current_key_chord {
            self.write.queue(Print(c))?;
        }
        self.write.queue(ResetColor)?;
        Ok(())
    }

    fn show_help(&mut self, header: &Header) -> Result<()> {
        self.show_header(header, HeaderKind::Ok)?;
        queue!(
            self.write,
            Print("Verco "),
            Print(VERSION),
            Print('\n'),
            Print('\n')
        )?;

        match self.version_control.version() {
            Ok(version) => {
                queue!(self.write, Print(version), Print('\n'), Print('\n'))?;
            }
            Err(error) => {
                self.show_header(header, HeaderKind::Error)?;
                self.write.queue(Print(error))?;
                return Ok(());
            }
        }

        queue!(self.write, Print("press a key and peform an action\n\n"))?;

        self.show_help_action("h", "help")?;
        self.show_help_action("q", "quit")?;

        self.write.queue(Print('\n'))?;

        self.show_help_action("s", "status")?;
        self.show_help_action("ll", "log 20")?;
        self.show_help_action("lc", "log count")?;

        self.write.queue(Print('\n'))?;

        self.show_help_action("dd", "revision diff")?;
        self.show_help_action("dc", "revision changes")?;

        self.write.queue(Print('\n'))?;

        self.show_help_action("cc", "commit all")?;
        self.show_help_action("cs", "commit selected")?;
        self.show_help_action("u", "update/checkout")?;
        self.show_help_action("m", "merge")?;
        self.show_help_action("RA", "revert all")?;
        self.show_help_action("rs", "revert selected")?;

        self.write.queue(Print('\n'))?;

        self.show_help_action("rr", "list unresolved conflicts")?;
        self.show_help_action("ro", "resolve taking other")?;
        self.show_help_action("rl", "resolve taking local")?;

        self.write.queue(Print('\n'))?;

        self.show_help_action("f", "fetch")?;
        self.show_help_action("p", "pull")?;
        self.show_help_action("P", "push")?;

        self.write.queue(Print('\n'))?;

        self.show_help_action("tn", "new tag")?;

        self.write.queue(Print('\n'))?;

        self.show_help_action("bb", "list branches")?;
        self.show_help_action("bn", "new branch")?;
        self.show_help_action("bd", "delete branch")?;

        self.write.queue(Print('\n'))?;

        self.show_help_action("x", "custom command")?;
        Ok(())
    }

    fn show_help_action(&mut self, shortcut: &str, action: &str) -> Result<()> {
        queue!(
            self.write,
            SetForegroundColor(ENTRY_COLOR),
            Print('\t'),
            Print(shortcut),
            ResetColor,
            Print('\t'),
            Print('\t'),
            Print(action),
            Print('\n'),
        )
    }
}
