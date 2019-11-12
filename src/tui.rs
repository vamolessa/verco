use crossterm::*;

use std::{borrow::BorrowMut, process::Command};

use crate::{
    custom_commands::CustomCommand,
    select::{select, Entry},
    version_control_actions::VersionControlActions,
};

const RESET_COLOR: Attribute = Attribute::Reset;
const HEADER_COLOR: Colored = Colored::Fg(Color::Black);
const HEADER_BG_COLOR: Colored = Colored::Bg(Color::Magenta);
const ACTION_COLOR: Colored = Colored::Fg(Color::Rgb {
    r: 255,
    g: 100,
    b: 180,
});
const ENTRY_COLOR: Colored = Colored::Fg(Color::Rgb {
    r: 255,
    g: 180,
    b: 100,
});

const DONE_COLOR: Colored = Colored::Fg(Color::Green);
const CANCEL_COLOR: Colored = Colored::Fg(Color::Yellow);
const ERROR_COLOR: Colored = Colored::Fg(Color::Red);

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub fn show_tui(
    version_controls: Vec<Box<dyn 'static + VersionControlActions>>,
    custom_commands: Vec<CustomCommand>,
) {
    Tui::new(version_controls, custom_commands).show();
}

struct Tui {
    version_controls: Vec<Box<dyn 'static + VersionControlActions>>,
    custom_commands: Vec<CustomCommand>,

    current_version_control_index: usize,
    current_key_chord: Vec<char>,

    _crossterm: Crossterm,
    terminal: Terminal,
    input: TerminalInput,
    cursor: TerminalCursor,
}

impl Tui {
    fn new(
        version_controls: Vec<Box<dyn 'static + VersionControlActions>>,
        custom_commands: Vec<CustomCommand>,
    ) -> Self {
        let crossterm = Crossterm::new();
        let terminal = crossterm.terminal();
        let input = crossterm.input();
        let cursor = crossterm.cursor();

        Tui {
            version_controls,
            custom_commands,
            current_version_control_index: 0,
            current_key_chord: Vec::new(),
            _crossterm: crossterm,
            terminal,
            input,
            cursor,
        }
    }

    fn current_version_control_mut(&mut self) -> &mut (dyn 'static + VersionControlActions) {
        self.version_controls[self.current_version_control_index].borrow_mut()
    }

    fn show(&mut self) {
        self.cursor.hide().unwrap();
        self.show_header();
        self.show_help();

        while self.handle_command() {
            self.current_key_chord.clear();
            self.show_current_key_chord();
        }

        self.cursor.show().unwrap();
    }

    fn next_key(&mut self) -> char {
        let mut ignore_next = false;
        loop {
            match self.input.read_char() {
                Ok(key) => {
                    self.terminal.clear(ClearType::CurrentLine).unwrap();
                    self.cursor.move_left(1);

                    if ignore_next {
                        ignore_next = false;
                        continue;
                    }

                    self.current_key_chord.push(key);
                    self.show_current_key_chord();
                    return key;
                }
                Err(_error) => {
                    ignore_next = true;
                }
            }
        }
    }

    fn handle_command(&mut self) -> bool {
        match self.next_key() {
            // ctrl+c or esc
            '\x03' | '\x1b' => return false,
            'h' => {
                self.show_action("help");
                self.show_help();
            }
            'e' => {
                self.show_action("explorer");
                self.open_explorer();
            }
            's' => {
                self.show_action("status");
                let result = self.current_version_control_mut().status();
                self.handle_result(result);
            }
            'l' => {
                self.show_action("log");
                let result = self.current_version_control_mut().log();
                self.handle_result(result);
            }
            'd' => match self.next_key() {
                'd' => {
                    self.show_action("revision diff");
                    if let Some(input) = self.handle_input("show diff from (ctrl+c to cancel): ") {
                        let result = self.current_version_control_mut().diff(&input[..]);
                        self.handle_result(result);
                    }
                }
                'c' => {
                    self.show_action("revision changes");
                    if let Some(input) = self.handle_input("show changes from (ctrl+c to cancel): ")
                    {
                        let result = self.current_version_control_mut().changes(&input[..]);
                        self.handle_result(result);
                    }
                }
                _ => {}
            },
            'c' => match self.next_key() {
                'c' => {
                    self.show_action("commit all");

                    if let Some(input) = self.handle_input("commit message (ctrl+c to cancel): ") {
                        let result = self.current_version_control_mut().commit_all(&input[..]);
                        self.handle_result(result);
                    }
                }
                'C' => {
                    self.show_action("commit selected");
                    match self.current_version_control_mut().get_files_to_commit() {
                        Ok(mut entries) => {
                            if self.show_select_ui(&mut entries) {
                                print!("\n\n");

                                if let Some(input) =
                                    self.handle_input("commit message (ctrl+c to cancel): ")
                                {
                                    let result = self
                                        .current_version_control_mut()
                                        .commit_selected(&input[..], &entries);
                                    self.handle_result(result);
                                }
                            }
                        }
                        Err(error) => self.handle_result(Err(error)),
                    }
                }
                _ => {}
            },
            'u' => {
                self.show_action("update");
                if let Some(input) = self.handle_input("update to (ctrl+c to cancel): ") {
                    let result = self.current_version_control_mut().update(&input[..]);
                    self.handle_result(result);
                }
            }
            'm' => {
                self.show_action("merge");
                if let Some(input) = self.handle_input("merge with (ctrl+c to cancel): ") {
                    let result = self.current_version_control_mut().merge(&input[..]);
                    self.handle_result(result);
                }
            }
            'R' => match self.next_key() {
                'a' | 'A' => {
                    self.show_action("revert all");
                    let result = self.current_version_control_mut().revert_all();
                    self.handle_result(result);
                }
                's' | 'S' => {
                    self.show_action("revert selected");
                    match self.current_version_control_mut().get_files_to_commit() {
                        Ok(mut entries) => {
                            if self.show_select_ui(&mut entries) {
                                print!("\n\n");
                                let result =
                                    self.current_version_control_mut().revert_selected(&entries);
                                self.handle_result(result);
                            }
                        }
                        Err(error) => self.handle_result(Err(error)),
                    }
                }
                _ => {}
            },
            'r' => match self.next_key() {
                'r' => {
                    self.show_action("unresolved conflicts");
                    let result = self.current_version_control_mut().conflicts();
                    self.handle_result(result);
                }
                'o' => {
                    self.show_action("merge taking other");
                    let result = self.current_version_control_mut().take_other();
                    self.handle_result(result);
                }
                'l' => {
                    self.show_action("merge taking local");
                    let result = self.current_version_control_mut().take_local();
                    self.handle_result(result);
                }
                _ => {}
            },
            'f' => {
                self.show_action("fetch");
                let result = self.current_version_control_mut().fetch();
                self.handle_result(result);
            }
            'p' => {
                self.show_action("pull");
                let result = self.current_version_control_mut().pull();
                self.handle_result(result);
            }
            'P' => {
                self.show_action("push");
                let result = self.current_version_control_mut().push();
                self.handle_result(result);
            }
            't' => match self.next_key() {
                'n' => {
                    self.show_action("create tag");
                    if let Some(input) = self.handle_input("tag name (ctrl+c to cancel): ") {
                        let result = self.current_version_control_mut().create_tag(&input[..]);
                        self.handle_result(result);
                    }
                }
                _ => {}
            },
            'b' => match self.next_key() {
                'b' => {
                    self.show_action("list branches");
                    let result = self.current_version_control_mut().list_branches();
                    self.handle_result(result);
                }
                'n' => {
                    self.show_action("create branch");
                    if let Some(input) = self.handle_input("branch name (ctrl+c to cancel): ") {
                        let result = self.current_version_control_mut().create_branch(&input[..]);
                        self.handle_result(result);
                    }
                }
                'd' => {
                    self.show_action("close branch");
                    if let Some(input) = self.handle_input("branch to close (ctrl+c to cancel): ") {
                        let result = self.current_version_control_mut().close_branch(&input[..]);
                        self.handle_result(result);
                    }
                }
                _ => {}
            },
            'x' => {
                self.show_action("custom command");
                println!("SDASD");
            }
            _ => (),
        }

        true
    }

    fn handle_input(&mut self, prompt: &str) -> Option<String> {
        print!("{}{}{}\n", ENTRY_COLOR, prompt, RESET_COLOR);
        self.cursor.show().unwrap();
        let res = match self.input.read_line() {
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
            print!("\n\n{}canceled{}\n\n", CANCEL_COLOR, RESET_COLOR);
        }

        self.cursor.hide().unwrap();
        res
    }

    fn handle_result(&mut self, result: std::result::Result<String, String>) {
        match result {
            Ok(output) => {
                print!("{}\n\n", output);
                print!("{}done{}\n\n", DONE_COLOR, RESET_COLOR);
            }
            Err(error) => {
                print!("{}\n\n", error);
                print!("{}error{}\n\n", ERROR_COLOR, RESET_COLOR);
            }
        }
    }

    fn show_header(&mut self) {
        self.terminal.clear(ClearType::All).unwrap();

        let (w, _) = self.terminal.terminal_size();
        self.cursor.goto(0, 0).unwrap();
        print!("{}{}", HEADER_COLOR, HEADER_BG_COLOR,);
        print!("{}", " ".repeat(w as usize));

        self.cursor.goto(0, 0).unwrap();
        print!("{}Verco @ ", HEADER_COLOR);

        if self.version_controls.len() > 1 {
            print!(
                "({}/{}) ",
                self.current_version_control_index + 1,
                self.version_controls.len()
            );
        }

        print!(
            "{}{}{}\n\n",
            self.current_version_control_mut().repository_directory(),
            RESET_COLOR,
            RESET_COLOR
        );
    }

    fn show_action(&mut self, action_name: &str) {
        self.show_header();
        print!("{}{}{}\n\n", ACTION_COLOR, action_name, RESET_COLOR);
    }

    fn show_current_key_chord(&mut self) {
        let (w, h) = self.terminal.terminal_size();
        self.cursor
            .goto(w - self.current_key_chord.len() as u16 - 2, h - 2)
            .unwrap();
        self.terminal.clear(ClearType::CurrentLine).unwrap();
        print!("{}", ACTION_COLOR);
        for k in &self.current_key_chord {
            print!("{}", k);
        }
        print!("{}\n", RESET_COLOR);
    }

    fn show_help(&mut self) {
        print!("Verco {}\n\n", VERSION);

        match self.current_version_control_mut().version() {
            Ok(version) => {
                print!("{}", version);
                print!("\n\n");
            }
            Err(error) => {
                print!("{}{}", ERROR_COLOR, error);
                panic!("Could not find version control in system");
            }
        }

        print!("press a key and peform an action\n\n");

        self.show_help_action("h", "help");
        self.show_help_action("e", "explorer\n");

        self.show_help_action("s", "status");
        self.show_help_action("l", "log\n");

        self.show_help_action("dd", "revision diff");
        self.show_help_action("dc", "revision changes\n");

        self.show_help_action("cc", "commit all");
        self.show_help_action("cs", "commit selected");
        self.show_help_action("u", "update/checkout");
        self.show_help_action("m", "merge");
        self.show_help_action("S-ra", "revert all");
        self.show_help_action("S-rs", "revert selected\n");

        self.show_help_action("rr", "unresolved conflicts");
        self.show_help_action("ro", "resolve taking other");
        self.show_help_action("rl", "resolve taking local\n");

        self.show_help_action("f", "fetch");
        self.show_help_action("p", "pull");
        self.show_help_action("S-p", "push\n");

        self.show_help_action("tn", "new tag\n");

        self.show_help_action("bb", "list branches");
        self.show_help_action("bn", "new branch");
        self.show_help_action("bd", "delete branch\n");

        self.show_help_action("x", "custom command\n");
    }

    fn show_help_action(&mut self, shortcut: &str, action: &str) {
        print!(
            "\t{}{}{}\t\t{}\n",
            ENTRY_COLOR, shortcut, RESET_COLOR, action
        );
    }

    fn open_explorer(&mut self) {
        let mut command = Command::new("explorer");
        command.arg(self.current_version_control_mut().repository_directory());
        command.spawn().expect("failed to open explorer");

        print!("{}done{}\n\n", DONE_COLOR, RESET_COLOR);
    }

    pub fn show_select_ui(&mut self, entries: &mut Vec<Entry>) -> bool {
        if select(
            &mut self.terminal,
            &mut self.cursor,
            &mut self.input,
            entries,
        ) {
            true
        } else {
            print!("\n\n{}canceled{}\n\n", CANCEL_COLOR, RESET_COLOR);
            false
        }
    }
}
