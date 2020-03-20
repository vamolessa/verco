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
    iter,
};

use crate::{
    ctrlc_handler::CtrlcHandler,
    custom_commands::CustomCommand,
    input,
    scroll_view::show_scroll_view,
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

enum HandleChordResult {
    Handled,
    Unhandled,
    Quit,
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
            cursor::MoveTo(w - 1, h - 1),
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
            ['h'] => {
                self.show_action("help")?;
                queue!(self.write, Print('\n'), Print('\n'))?;
                self.show_help()?;
                Ok(HandleChordResult::Handled)
            }
            ['s'] => {
                self.show_action("status")?;
                let result = self.current_version_control_mut().status();
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['l'] => Ok(HandleChordResult::Unhandled),
            ['l', 'l'] => {
                self.show_action("log")?;
                let result = self.current_version_control_mut().log(20);
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['d'] => Ok(HandleChordResult::Unhandled),
            ['d', 'd'] => {
                self.show_action("revision diff")?;
                queue!(self.write, Print('\n'), Print('\n'))?;
                if let Some(input) = self.handle_input("show diff from (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().diff(&input[..]);
                    self.handle_result(result)?;
                }
                Ok(HandleChordResult::Handled)
            }
            ['d', 'c'] => {
                self.show_action("revision changes")?;
                queue!(self.write, Print('\n'), Print('\n'))?;
                if let Some(input) = self.handle_input("show changes from (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().changes(&input[..]);
                    self.handle_result(result)?;
                }
                Ok(HandleChordResult::Handled)
            }
            ['c'] => Ok(HandleChordResult::Unhandled),
            ['c', 'c'] => {
                self.show_action("commit all")?;
                queue!(self.write, Print('\n'), Print('\n'))?;
                if let Some(input) = self.handle_input("commit message (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().commit_all(&input[..]);
                    self.handle_result(result)?;
                }
                Ok(HandleChordResult::Handled)
            }
            ['c', 's'] => {
                self.show_action("commit selected")?;
                match self.current_version_control_mut().get_files_to_commit() {
                    Ok(mut entries) => {
                        if self.show_select_ui(&mut entries)? {
                            queue!(self.write, Print('\n'), Print('\n'))?;
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
                Ok(HandleChordResult::Handled)
            }
            ['u'] => {
                self.show_action("update")?;
                queue!(self.write, Print('\n'), Print('\n'))?;
                if let Some(input) = self.handle_input("update to (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().update(&input[..]);
                    self.handle_result(result)?;
                }
                Ok(HandleChordResult::Handled)
            }
            ['m'] => {
                self.show_action("merge")?;
                queue!(self.write, Print('\n'), Print('\n'))?;
                if let Some(input) = self.handle_input("merge with (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().merge(&input[..]);
                    self.handle_result(result)?;
                }
                Ok(HandleChordResult::Handled)
            }
            ['R'] => Ok(HandleChordResult::Unhandled),
            ['R', 'a'] | ['R', 'A'] => {
                self.show_action("revert all")?;
                let result = self.current_version_control_mut().revert_all();
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['r'] => Ok(HandleChordResult::Unhandled),
            ['r', 's'] => {
                self.show_action("revert selected")?;
                match self.current_version_control_mut().get_files_to_commit() {
                    Ok(mut entries) => {
                        if self.show_select_ui(&mut entries)? {
                            queue!(self.write, Print('\n'), Print('\n'))?;
                            let result =
                                self.current_version_control_mut().revert_selected(&entries);
                            self.handle_result(result)?;
                        }
                    }
                    Err(error) => self.handle_result(Err(error))?,
                }
                Ok(HandleChordResult::Handled)
            }
            ['r', 'r'] => {
                self.show_action("unresolved conflicts")?;
                let result = self.current_version_control_mut().conflicts();
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['r', 'o'] => {
                self.show_action("merge taking other")?;
                let result = self.current_version_control_mut().take_other();
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['r', 'l'] => {
                self.show_action("merge taking local")?;
                let result = self.current_version_control_mut().take_local();
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['f'] => {
                self.show_action("fetch")?;
                let result = self.current_version_control_mut().fetch();
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['p'] => {
                self.show_action("pull")?;
                let result = self.current_version_control_mut().pull();
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['P'] => {
                self.show_action("push")?;
                let result = self.current_version_control_mut().push();
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['t'] => Ok(HandleChordResult::Unhandled),
            ['t', 'n'] => {
                self.show_action("new tag")?;
                queue!(self.write, Print('\n'), Print('\n'))?;
                if let Some(input) = self.handle_input("new tag name (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().create_tag(&input[..]);
                    self.handle_result(result)?;
                }
                Ok(HandleChordResult::Handled)
            }
            ['b'] => Ok(HandleChordResult::Unhandled),
            ['b', 'b'] => {
                self.show_action("list branches")?;
                let result = self.current_version_control_mut().list_branches();
                self.handle_result(result)?;
                Ok(HandleChordResult::Handled)
            }
            ['b', 'n'] => {
                self.show_action("new branch")?;
                queue!(self.write, Print('\n'), Print('\n'))?;
                if let Some(input) = self.handle_input("new branch name (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().create_branch(&input[..]);
                    self.handle_result(result)?;
                }
                Ok(HandleChordResult::Handled)
            }
            ['b', 'd'] => {
                self.show_action("delete branch")?;
                queue!(self.write, Print('\n'), Print('\n'))?;
                if let Some(input) = self.handle_input("branch to delete (ctrl+c to cancel): ")? {
                    let result = self.current_version_control_mut().close_branch(&input[..]);
                    self.handle_result(result)?;
                }
                Ok(HandleChordResult::Handled)
            }
            ['x'] => {
                self.show_action("custom command")?;
                if self.custom_commands.len() > 0 {
                    queue!(self.write, ResetColor, Print("\n\navailable commands\n\n"))?;
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
                    self.current_key_chord.clear();
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
                Ok(HandleChordResult::Handled)
            }
            _ => Ok(HandleChordResult::Handled),
        }
    }

    fn handle_custom_command(&mut self) -> Result<()> {
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
                    queue!(
                        self.write,
                        cursor::RestorePosition,
                        SetForegroundColor(CANCEL_COLOR),
                        Print("\n\ncanceled\n\n"),
                        ResetColor
                    )?;
                    return Ok(());
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
                                .queue(SetForegroundColor(ACTION_COLOR))?
                                .queue(Print(&command.command))?
                                .queue(ResetColor)?;
                            for arg in &command.args {
                                self.write.queue(Print(' '))?.queue(Print(arg))?;
                            }
                            self.write.queue(Print('\n'))?.queue(Print('\n'))?;

                            let result = command
                                .execute(self.current_version_control().repository_directory());
                            self.handle_result(result)?;
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

                    queue!(
                        self.write,
                        cursor::RestorePosition,
                        Print('\n'),
                        Print('\n'),
                        SetForegroundColor(CANCEL_COLOR),
                        Print("no match found\n\n"),
                        ResetColor
                    )?;
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
        queue!(self.write, Print('\n'), Print('\n'))?;
        show_scroll_view(&mut self.write, &mut self.ctrlc_handler, result)
        // match result {
        //     Ok(output) => queue!(self.write, Print(output)),
        //     Err(error) => queue!(self.write, SetForegroundColor(ERROR_COLOR), Print(error),),
        // }
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
            Print('\n'),
            Print('\n'),
        )?;
        Ok(())
    }

    fn show_action(&mut self, action_name: &str) -> Result<()> {
        self.show_header()?;
        queue!(
            self.write,
            SetForegroundColor(ACTION_COLOR),
            Print(action_name),
            ResetColor,
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
        for c in &self.current_key_chord {
            self.write.queue(Print(c))?;
        }
        self.write.queue(ResetColor)?;
        Ok(())
    }

    fn show_help(&mut self) -> Result<()> {
        queue!(self.write, Print(format!("Verco {}\n\n", VERSION)))?;

        match self.current_version_control_mut().version() {
            Ok(version) => {
                queue!(self.write, Print(version), Print('\n'), Print('\n'))?;
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
            Print('\t'),
            Print('\t'),
            Print(action),
            Print('\n'),
        )
    }

    fn show_select_ui(&mut self, entries: &mut Vec<Entry>) -> Result<bool> {
        queue!(self.write, Print('\n'), Print('\n'))?;
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
