use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    QueueableCommand, Result,
};

use std::{
    borrow::{Borrow, BorrowMut},
    io::{stdout, Write},
};

use crate::{
    ctrlc_handler::CtrlcHandler,
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
    ctrlc_handler: CtrlcHandler,
) {
    Tui::new(
        version_controls,
        custom_commands,
        stdout().lock(),
        ctrlc_handler,
    )
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

    write: W,
    ctrlc_handler: CtrlcHandler,
}

impl<W> Tui<W>
where
    W: Write,
{
    fn new(
        version_controls: Vec<Box<dyn 'static + VersionControlActions>>,
        custom_commands: Vec<CustomCommand>,
        write: W,
        ctrlc_handler: CtrlcHandler,
    ) -> Self {
        Tui {
            version_controls,
            custom_commands,
            current_version_control_index: 0,
            current_key_chord: Vec::new(),
            write,
            ctrlc_handler,
        }
    }

    fn current_version_control(&self) -> &(dyn 'static + VersionControlActions) {
        self.version_controls[self.current_version_control_index].borrow()
    }

    fn current_version_control_mut(&mut self) -> &mut (dyn 'static + VersionControlActions) {
        self.version_controls[self.current_version_control_index].borrow_mut()
    }

    fn show(&mut self) -> Result<()> {
        queue!(self.write, cursor::Hide)?;
        self.show_header()?;
        self.show_help()?;
        let (w, h) = terminal::size()?;
        queue!(
            self.write,
            cursor::MoveTo(w - 2, h - 2),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ACTION_COLOR),
        )?;
        self.write.flush()?;

        while self.handle_command()? {
            self.current_key_chord.clear();
            self.show_current_key_chord()?;
        }

        execute!(self.write, ResetColor, cursor::Show)?;
        Ok(())
    }

    fn handle_command(&mut self) -> Result<bool> {
        self.write.flush()?;
        match input::read_key(&mut self.ctrlc_handler)? {
            KeyEvent {
                code: KeyCode::Esc, ..
            }
            | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            } => return Ok(false),
            key_event => match input::key_to_char(key_event) {
                'q' => return Ok(false),
                'h' => {
                    self.show_action("help")?;
                    self.show_help()?;
                }
                's' => {
                    self.show_action("status")?;
                    let result = self.current_version_control_mut().status();
                    self.handle_result(result)?;
                }
                'l' => match input::key_to_char(input::read_key(&mut self.ctrlc_handler)?) {
                    'l' => {
                        self.show_action("log")?;
                        let result = self.current_version_control_mut().log();
                        self.handle_result(result)?;
                    }
                    _ => (),
                },
                'd' => match input::key_to_char(input::read_key(&mut self.ctrlc_handler)?) {
                    'd' => {
                        self.show_action("revision diff")?;
                        if let Some(input) =
                            self.handle_input("show diff from (ctrl+c to cancel): ")?
                        {
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
                'c' => match input::key_to_char(input::read_key(&mut self.ctrlc_handler)?) {
                    'c' => {
                        self.show_action("commit all")?;

                        if let Some(input) =
                            self.handle_input("commit message (ctrl+c to cancel): ")?
                        {
                            let result = self.current_version_control_mut().commit_all(&input[..]);
                            self.handle_result(result)?;
                        }
                    }
                    's' => {
                        self.show_action("commit selected")?;
                        match self.current_version_control_mut().get_files_to_commit() {
                            Ok(mut entries) => {
                                if self.show_select_ui(&mut entries)? {
                                    queue!(self.write, Print("\n\n"))?;

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
                'R' => match input::key_to_char(input::read_key(&mut self.ctrlc_handler)?) {
                    'a' | 'A' => {
                        self.show_action("revert all")?;
                        let result = self.current_version_control_mut().revert_all();
                        self.handle_result(result)?;
                    }
                    _ => (),
                },
                'r' => match input::key_to_char(input::read_key(&mut self.ctrlc_handler)?) {
                    's' => {
                        self.show_action("revert selected")?;
                        match self.current_version_control_mut().get_files_to_commit() {
                            Ok(mut entries) => {
                                if self.show_select_ui(&mut entries)? {
                                    queue!(self.write, Print("\n\n"))?;
                                    let result = self
                                        .current_version_control_mut()
                                        .revert_selected(&entries);
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
                't' => match input::key_to_char(input::read_key(&mut self.ctrlc_handler)?) {
                    'n' => {
                        self.show_action("new tag")?;
                        if let Some(input) =
                            self.handle_input("new tag name (ctrl+c to cancel): ")?
                        {
                            let result = self.current_version_control_mut().create_tag(&input[..]);
                            self.handle_result(result)?;
                        }
                    }
                    _ => (),
                },
                'b' => match input::key_to_char(input::read_key(&mut self.ctrlc_handler)?) {
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
                            let result =
                                self.current_version_control_mut().create_branch(&input[..]);
                            self.handle_result(result)?;
                        }
                    }
                    'd' => {
                        self.show_action("delete branch")?;
                        if let Some(input) =
                            self.handle_input("branch to delete (ctrl+c to cancel): ")?
                        {
                            let result =
                                self.current_version_control_mut().close_branch(&input[..]);
                            self.handle_result(result)?;
                        }
                    }
                    _ => (),
                },
                'x' => {
                    self.show_action("custom command")?;
                    if self.custom_commands.len() > 0 {
                        queue!(self.write, ResetColor, Print("available commands\n\n"))?;
                        for c in &self.custom_commands {
                            self.write
                                .queue(SetForegroundColor(ENTRY_COLOR))?
                                .queue(Print('\t'))?
                                .queue(Print(&c.shortcut))?
                                .queue(Print("\t\t"))?
                                .queue(ResetColor)?
                                .queue(Print(&c.command))?;
                            for a in &c.args {
                                self.write.queue(Print(' '))?.queue(Print(a))?;
                            }
                            self.write.queue(Print('\n'))?;
                        }
                        self.handle_custom_command()?;
                    } else {
                        queue!(
                            self.write,
                            ResetColor,
                            Print("no commands available\n\n"),
                            Print(
                                "create commands by placing them inside './verco/custom_commands.txt'"
                            )
                        )?;
                    }
                }
                _ => (),
            },
        }

        Ok(true)
    }

    fn handle_custom_command(&mut self) -> Result<()> {
        let mut current_key_chord = String::new();
        queue!(self.write, cursor::SavePosition)?;

        'outer: loop {
            let key_event = input::read_key(&mut self.ctrlc_handler)?;
            match key_event {
                KeyEvent {
                    code: KeyCode::Esc, ..
                }
                | KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                } => {
                    queue!(
                        self.write,
                        cursor::RestorePosition,
                        SetForegroundColor(CANCEL_COLOR),
                        Print("\n\ncanceled\n\n"),
                        ResetColor
                    )?;
                    return Ok(());
                }
                _ => (),
            }

            let key = input::key_to_char(key_event);
            current_key_chord.push(key);
            for c in &self.custom_commands {
                if c.shortcut == current_key_chord {
                    self.write
                        .queue(cursor::RestorePosition)?
                        .queue(SetForegroundColor(ACTION_COLOR))?
                        .queue(Print("\n\n"))?
                        .queue(Print(&c.command))?
                        .queue(ResetColor)?;
                    for a in &c.args {
                        self.write.queue(Print(' '))?.queue(Print(a))?;
                    }
                    self.write.queue(Print("\n\n"))?;

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
                self.write,
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
            self.write,
            SetForegroundColor(ENTRY_COLOR),
            Print(prompt),
            ResetColor,
            Print("\n"),
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

        if res.is_none() {
            queue!(
                self.write,
                SetForegroundColor(CANCEL_COLOR),
                Print("\n\ncanceled\n\n"),
                ResetColor
            )?;
        }

        execute!(self.write, cursor::Hide)?;
        Ok(res)
    }

    fn handle_result(&mut self, result: std::result::Result<String, String>) -> Result<()> {
        match result {
            Ok(output) => queue!(
                self.write,
                Print(output),
                Print("\n\n"),
                SetForegroundColor(DONE_COLOR),
                Print("done"),
                ResetColor,
                Print("\n\n")
            ),
            Err(error) => queue!(
                self.write,
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

        let header = "Verco @ ";

        queue!(
            self.write,
            Clear(ClearType::All),
            cursor::MoveTo(0, 0),
            SetBackgroundColor(HEADER_BG_COLOR),
            SetForegroundColor(HEADER_COLOR),
            Print(header),
        )?;

        let directory_name = self
            .current_version_control()
            .repository_directory()
            .to_owned();
        let width_left = w as usize - 1 - header.len() - directory_name.len();

        queue!(
            self.write,
            Print(directory_name),
            Print(" ".repeat(width_left)),
            ResetColor,
            Print("\n\n")
        )?;
        Ok(())
    }

    fn show_action(&mut self, action_name: &str) -> Result<()> {
        self.show_header()?;
        queue!(
            self.write,
            SetForegroundColor(ACTION_COLOR),
            Print(action_name),
            Print("\n\n"),
            ResetColor
        )?;
        self.write.flush()?;
        Ok(())
    }

    fn show_current_key_chord(&mut self) -> Result<()> {
        let (w, h) = terminal::size()?;
        queue!(
            self.write,
            cursor::MoveTo(w - self.current_key_chord.len() as u16 - 2, h - 2),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(ACTION_COLOR),
        )?;
        for k in &self.current_key_chord {
            self.write.queue(Print(k))?;
        }
        queue!(self.write, ResetColor)?;
        Ok(())
    }

    fn show_help(&mut self) -> Result<()> {
        queue!(self.write, Print(format!("Verco {}\n\n", VERSION)))?;

        match self.current_version_control_mut().version() {
            Ok(version) => {
                queue!(self.write, Print(version), Print("\n\n"))?;
            }
            Err(error) => {
                queue!(
                    self.write,
                    SetForegroundColor(ERROR_COLOR),
                    Print(error),
                    Print("Could not find version control in system")
                )?;
            }
        }

        queue!(self.write, Print("press a key and peform an action\n\n"))?;

        self.show_help_action("h", "help")?;
        self.show_help_action("q", "quit\n")?;

        self.show_help_action("s", "status")?;
        self.show_help_action("ll", "log\n")?;

        self.show_help_action("dd", "revision diff")?;
        self.show_help_action("dc", "revision changes\n")?;

        self.show_help_action("cc", "commit all")?;
        self.show_help_action("cs", "commit selected")?;
        self.show_help_action("u", "update/checkout")?;
        self.show_help_action("m", "merge")?;
        self.show_help_action("RA", "revert all")?;
        self.show_help_action("rs", "revert selected\n")?;

        self.show_help_action("rr", "list unresolved conflicts")?;
        self.show_help_action("ro", "resolve taking other")?;
        self.show_help_action("rl", "resolve taking local\n")?;

        self.show_help_action("f", "fetch")?;
        self.show_help_action("p", "pull")?;
        self.show_help_action("P", "push\n")?;

        self.show_help_action("tn", "new tag\n")?;

        self.show_help_action("bb", "list branches")?;
        self.show_help_action("bn", "new branch")?;
        self.show_help_action("bd", "delete branch\n")?;

        self.show_help_action("x", "custom command\n")?;
        Ok(())
    }

    fn show_help_action(&mut self, shortcut: &str, action: &str) -> Result<()> {
        queue!(
            self.write,
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
        if select(&mut self.write, &mut self.ctrlc_handler, entries)? {
            Ok(true)
        } else {
            queue!(
                self.write,
                SetForegroundColor(CANCEL_COLOR),
                Print("\n\ncanceled\n\n"),
                ResetColor
            )?;
            Ok(false)
        }
    }
}
