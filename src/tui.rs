use crossterm::{
    cursor, execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    QueueableCommand, Result,
};

use std::{
    borrow::{Borrow, BorrowMut},
    io::{stdout, Write},
};

use crate::{
    custom_commands::CustomCommand,
    input,
    select::{select, Entry},
    version_control_actions::VersionControlActions,
};

const HEADER_COLOR: Color = Color::Black;
const HEADER_BG_COLOR: Color = Color::Magenta;
const ACTION_COLOR: Color = Color::Rgb {
    r: 255,
    g: 100,
    b: 180,
};
const ENTRY_COLOR: Color = Color::Rgb {
    r: 255,
    g: 180,
    b: 100,
};

const DONE_COLOR: Color = Color::Green;
const CANCEL_COLOR: Color = Color::Yellow;
const ERROR_COLOR: Color = Color::Red;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn show_tui(
    version_controls: Vec<Box<dyn 'static + VersionControlActions>>,
    custom_commands: Vec<CustomCommand>,
) {
    Tui::new(version_controls, custom_commands, stdout().lock())
        .show()
        .unwrap();
}

struct Tui<W>
where
    W: Write,
{
    version_controls: Vec<Box<dyn 'static + VersionControlActions>>,
    custom_commands: Vec<CustomCommand>,

    current_version_control_index: usize,
    current_key_chord: Vec<char>,

    stdout: W,
}

impl<W> Tui<W>
where
    W: Write,
{
    fn new(
        version_controls: Vec<Box<dyn 'static + VersionControlActions>>,
        custom_commands: Vec<CustomCommand>,
        stdout: W,
    ) -> Self {
        Tui {
            version_controls,
            custom_commands,
            current_version_control_index: 0,
            current_key_chord: Vec::new(),
            stdout,
        }
    }

    fn current_version_control(&self) -> &(dyn 'static + VersionControlActions) {
        self.version_controls[self.current_version_control_index].borrow()
    }

    fn current_version_control_mut(&mut self) -> &mut (dyn 'static + VersionControlActions) {
        self.version_controls[self.current_version_control_index].borrow_mut()
    }

    fn show(&mut self) -> Result<()> {
        queue!(self.stdout, cursor::Hide)?;
        self.show_header()?;
        self.show_help()?;
        let (w, h) = terminal::size()?;
        queue!(
            self.stdout,
            cursor::MoveTo(w - 2, h - 2),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ACTION_COLOR),
        )?;
        self.stdout.flush()?;

        while self.handle_command()? {
            self.current_key_chord.clear();
            self.show_current_key_chord()?;
        }

        execute!(self.stdout, cursor::Show)?;
        Ok(())
    }

    fn next_key(&mut self) -> Result<char> {
        let mut ignore_next = false;
        loop {
            self.stdout.flush()?;
            match input::read_char() {
                Ok(key) => {
                    queue!(
                        self.stdout,
                        Clear(ClearType::CurrentLine),
                        cursor::MoveLeft(1)
                    )?;

                    if ignore_next {
                        ignore_next = false;
                        continue;
                    }

                    self.current_key_chord.push(key);
                    self.show_current_key_chord()?;
                    return Ok(key);
                }
                Err(_error) => {
                    ignore_next = true;
                }
            }
        }
    }

    fn handle_command(&mut self) -> Result<bool> {
        match self.next_key()? {
            // q or ctrl+c or esc
            'q' | '\x03' | '\x1b' => return Ok(false),
            'h' => {
                self.show_action("help")?;
                self.show_help()?;
            }
            's' => {
                self.show_action("status")?;
                let result = self.current_version_control_mut().status();
                self.handle_result(result)?;
            }
            'l' => {
                self.show_action("log")?;
                let result = self.current_version_control_mut().log();
                self.handle_result(result)?;
            }
            'd' => match self.next_key()? {
                'd' => {
                    self.show_action("revision diff")?;
                    if let Some(input) = self.handle_input("show diff from (ctrl+c to cancel): ")? {
                        let result = self.current_version_control_mut().diff(&input[..]);
                        self.handle_result(result)?;
                    }
                }
                'c' => {
                    self.show_action("revision changes")?;
                    if let Some(input) =
                        self.handle_input("show changes from (ctrl+c to cancel): ")?
                    {
                        let result = self.current_version_control_mut().changes(&input[..]);
                        self.handle_result(result)?;
                    }
                }
                _ => (),
            },
            'c' => match self.next_key()? {
                'c' => {
                    self.show_action("commit all")?;

                    if let Some(input) = self.handle_input("commit message (ctrl+c to cancel): ")? {
                        let result = self.current_version_control_mut().commit_all(&input[..]);
                        self.handle_result(result)?;
                    }
                }
                's' => {
                    self.show_action("commit selected")?;
                    match self.current_version_control_mut().get_files_to_commit() {
                        Ok(mut entries) => {
                            if self.show_select_ui(&mut entries)? {
                                queue!(self.stdout, Print("\n\n"))?;

                                if let Some(input) =
                                    self.handle_input("commit message (ctrl+c to cancel): ")?
                                {
                                    let result = self
                                        .current_version_control_mut()
                                        .commit_selected(&input[..], &entries);
                                    self.handle_result(result)?;
                                }
                            }
                        }
                        Err(error) => self.handle_result(Err(error))?,
                    }
                }
                _ => (),
            },
            'u' => {
                self.show_action("update")?;
                if let Some(input) = self.handle_input("update to (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().update(&input[..]);
                    self.handle_result(result)?;
                }
            }
            'm' => {
                self.show_action("merge")?;
                if let Some(input) = self.handle_input("merge with (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().merge(&input[..]);
                    self.handle_result(result)?;
                }
            }
            'R' => match self.next_key()? {
                'a' | 'A' => {
                    self.show_action("revert all")?;
                    let result = self.current_version_control_mut().revert_all();
                    self.handle_result(result)?;
                }
                _ => (),
            },
            'r' => match self.next_key()? {
                's' => {
                    self.show_action("revert selected")?;
                    match self.current_version_control_mut().get_files_to_commit() {
                        Ok(mut entries) => {
                            if self.show_select_ui(&mut entries)? {
                                queue!(self.stdout, Print("\n\n"))?;
                                let result =
                                    self.current_version_control_mut().revert_selected(&entries);
                                self.handle_result(result)?;
                            }
                        }
                        Err(error) => self.handle_result(Err(error))?,
                    }
                }
                'r' => {
                    self.show_action("unresolved conflicts")?;
                    let result = self.current_version_control_mut().conflicts();
                    self.handle_result(result)?;
                }
                'o' => {
                    self.show_action("merge taking other")?;
                    let result = self.current_version_control_mut().take_other();
                    self.handle_result(result)?;
                }
                'l' => {
                    self.show_action("merge taking local")?;
                    let result = self.current_version_control_mut().take_local();
                    self.handle_result(result)?;
                }
                _ => (),
            },
            'f' => {
                self.show_action("fetch")?;
                let result = self.current_version_control_mut().fetch();
                self.handle_result(result)?;
            }
            'p' => {
                self.show_action("pull")?;
                let result = self.current_version_control_mut().pull();
                self.handle_result(result)?;
            }
            'P' => {
                self.show_action("push")?;
                let result = self.current_version_control_mut().push();
                self.handle_result(result)?;
            }
            't' => match self.next_key()? {
                'n' => {
                    self.show_action("new tag")?;
                    if let Some(input) = self.handle_input("new tag name (ctrl+c to cancel): ")? {
                        let result = self.current_version_control_mut().create_tag(&input[..]);
                        self.handle_result(result)?;
                    }
                }
                _ => (),
            },
            'b' => match self.next_key()? {
                'b' => {
                    self.show_action("list branches")?;
                    let result = self.current_version_control_mut().list_branches();
                    self.handle_result(result)?;
                }
                'n' => {
                    self.show_action("new branch")?;
                    if let Some(input) =
                        self.handle_input("new branch name (ctrl+c to cancel): ")?
                    {
                        let result = self.current_version_control_mut().create_branch(&input[..]);
                        self.handle_result(result)?;
                    }
                }
                'd' => {
                    self.show_action("delete branch")?;
                    if let Some(input) =
                        self.handle_input("branch to delete (ctrl+c to cancel): ")?
                    {
                        let result = self.current_version_control_mut().close_branch(&input[..]);
                        self.handle_result(result)?;
                    }
                }
                _ => (),
            },
            'x' => {
                self.show_action("custom command")?;
                if self.custom_commands.len() > 0 {
                    queue!(self.stdout, ResetColor, Print("available commands\n\n"))?;
                    for c in &self.custom_commands {
                        self.stdout
                            .queue(SetForegroundColor(ENTRY_COLOR))?
                            .queue(Print('\t'))?
                            .queue(Print(&c.shortcut))?
                            .queue(Print("\t\t"))?
                            .queue(ResetColor)?
                            .queue(Print(&c.command))?;
                        for a in &c.args {
                            self.stdout.queue(Print(' '))?.queue(Print(a))?;
                        }
                        self.stdout.queue(Print('\n'))?;
                    }
                    self.handle_custom_command()?;
                } else {
                    queue!(
                        self.stdout,
                        ResetColor,
                        Print("no commands available\n\n"),
                        Print(
                            "create commands by placing them inside './verco/custom_commands.txt'"
                        )
                    )?;
                }
            }
            _ => (),
        }

        Ok(true)
    }

    fn handle_custom_command(&mut self) -> Result<()> {
        let mut current_key_chord = String::new();
        queue!(self.stdout, cursor::SavePosition)?;

        'outer: loop {
            let key = self.next_key()?;
            // ctrl+c or esc
            if key == '\x03' || key == '\x1b' {
                queue!(
                    self.stdout,
                    cursor::RestorePosition,
                    SetForegroundColor(CANCEL_COLOR),
                    Print("\n\ncanceled\n\n"),
                    ResetColor
                )?;
                return Ok(());
            }

            current_key_chord.push(key);
            for c in &self.custom_commands {
                if c.shortcut == current_key_chord {
                    self.stdout
                        .queue(cursor::RestorePosition)?
                        .queue(SetForegroundColor(ACTION_COLOR))?
                        .queue(Print("\n\n"))?
                        .queue(Print(&c.command))?
                        .queue(ResetColor)?;
                    for a in &c.args {
                        self.stdout.queue(Print(' '))?.queue(Print(a))?;
                    }
                    self.stdout.queue(Print("\n\n"))?;

                    let result = c.execute(self.current_version_control().repository_directory());
                    self.handle_result(result)?;
                    return Ok(());
                }
            }

            for c in &self.custom_commands {
                if c.shortcut.starts_with(&current_key_chord) {
                    continue 'outer;
                }
            }

            queue!(
                self.stdout,
                cursor::RestorePosition,
                SetForegroundColor(CANCEL_COLOR),
                Print("\n\nno match found\n\n"),
                ResetColor
            )?;
            return Ok(());
        }
    }

    fn handle_input(&mut self, prompt: &str) -> Result<Option<String>> {
        execute!(
            self.stdout,
            SetForegroundColor(ENTRY_COLOR),
            Print(prompt),
            ResetColor,
            Print("\n"),
            cursor::Show,
        )?;

        self.stdout.flush()?;

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

        if res.is_none() {
            queue!(
                self.stdout,
                SetForegroundColor(CANCEL_COLOR),
                Print("\n\ncanceled\n\n"),
                ResetColor
            )?;
        }

        execute!(self.stdout, cursor::Hide)?;
        Ok(res)
    }

    fn handle_result(&mut self, result: std::result::Result<String, String>) -> Result<()> {
        match result {
            Ok(output) => queue!(
                self.stdout,
                Print(output),
                Print("\n\n"),
                SetForegroundColor(DONE_COLOR),
                Print("done"),
                ResetColor,
                Print("\n\n")
            ),
            Err(error) => queue!(
                self.stdout,
                Print(error),
                Print("\n\n"),
                SetForegroundColor(ERROR_COLOR),
                Print("error"),
                ResetColor,
                Print("\n\n")
            ),
        }
    }

    fn show_header(&mut self) -> Result<()> {
        let (w, _) = terminal::size()?;

        queue!(
            self.stdout,
            Clear(ClearType::All),
            cursor::MoveTo(0, 0),
            SetBackgroundColor(HEADER_BG_COLOR),
            SetForegroundColor(HEADER_COLOR),
            Print(" ".repeat(w as usize)),
            cursor::MoveTo(0, 0),
            Print("Verco @ ")
        )?;

        if self.version_controls.len() > 1 {
            queue!(
                self.stdout,
                Print(self.current_version_control_index + 1),
                Print('/'),
                Print(self.version_controls.len())
            )?;
        }

        queue!(
            self.stdout,
            Print(
                self.current_version_control_mut()
                    .repository_directory()
                    .to_owned()
            ),
            ResetColor,
            Print("\n\n")
        )?;
        Ok(())
    }

    fn show_action(&mut self, action_name: &str) -> Result<()> {
        self.show_header()?;
        queue!(
            self.stdout,
            SetForegroundColor(ACTION_COLOR),
            Print(action_name),
            Print("\n\n"),
            ResetColor
        )?;
        Ok(())
    }

    fn show_current_key_chord(&mut self) -> Result<()> {
        let (w, h) = terminal::size()?;
        queue!(
            self.stdout,
            cursor::MoveTo(w - self.current_key_chord.len() as u16 - 2, h - 2),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ACTION_COLOR),
        )?;
        for k in &self.current_key_chord {
            self.stdout.queue(Print(k))?;
        }
        queue!(self.stdout, ResetColor)?;
        Ok(())
    }

    fn show_help(&mut self) -> Result<()> {
        queue!(self.stdout, Print(format!("Verco {}\n\n", VERSION)))?;

        match self.current_version_control_mut().version() {
            Ok(version) => {
                queue!(self.stdout, Print(version), Print("\n\n"))?;
            }
            Err(error) => {
                queue!(
                    self.stdout,
                    SetForegroundColor(ERROR_COLOR),
                    Print(error),
                    Print("Could not find version control in system")
                )?;
            }
        }

        queue!(self.stdout, Print("press a key and peform an action\n\n"))?;

        self.show_help_action("h", "help")?;
        self.show_help_action("q", "quit\n")?;

        self.show_help_action("s", "status")?;
        self.show_help_action("l", "log\n")?;

        self.show_help_action("dd", "revision diff")?;
        self.show_help_action("dc", "revision changes\n")?;

        self.show_help_action("cc", "commit all")?;
        self.show_help_action("cs", "commit selected")?;
        self.show_help_action("u", "update/checkout")?;
        self.show_help_action("m", "merge")?;
        self.show_help_action("S-ra", "revert all")?;
        self.show_help_action("rs", "revert selected\n")?;

        self.show_help_action("rr", "list unresolved conflicts")?;
        self.show_help_action("ro", "resolve taking other")?;
        self.show_help_action("rl", "resolve taking local\n")?;

        self.show_help_action("f", "fetch")?;
        self.show_help_action("p", "pull")?;
        self.show_help_action("S-p", "push\n")?;

        self.show_help_action("tn", "new tag\n")?;

        self.show_help_action("bb", "list branches")?;
        self.show_help_action("bn", "new branch")?;
        self.show_help_action("bd", "delete branch\n")?;

        self.show_help_action("x", "custom command\n")?;
        Ok(())
    }

    fn show_help_action(&mut self, shortcut: &str, action: &str) -> Result<()> {
        queue!(
            self.stdout,
            SetForegroundColor(ENTRY_COLOR),
            Print('\t'),
            Print(shortcut),
            ResetColor,
            Print("\t\t"),
            Print(action),
            Print('\n'),
        )
    }

    pub fn show_select_ui(&mut self, entries: &mut Vec<Entry>) -> Result<bool> {
        if select(&mut self.stdout, entries)? {
            Ok(true)
        } else {
            queue!(
                self.stdout,
                SetForegroundColor(CANCEL_COLOR),
                Print("\n\ncanceled\n\n"),
                ResetColor
            )?;
            Ok(false)
        }
    }
}
