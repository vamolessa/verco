use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader},
    process::Command,
};

use crate::action::ActionResult;

pub struct CustomAction {
    pub shortcut: String,
    pub command: String,
    pub args: Vec<String>,
}

impl CustomAction {
    pub fn load_custom_actions() -> Vec<CustomAction> {
        Self::try_load_custom_actions().unwrap_or(Vec::new())
    }

    fn try_load_custom_actions() -> io::Result<Vec<CustomAction>> {
        let mut path = env::current_dir()?;
        path.push(".verco/custom_actions.txt");
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut actions = Vec::new();
        let mut line = String::new();
        while next_line(&mut reader, &mut line) {
            let line = line.trim();
            if line.len() == 0 {
                continue;
            }

            let mut it = line.split(' ');

            let shortcut = it.nth(0);
            if shortcut.is_none() {
                continue;
            }

            let command = it.nth(0);
            if command.is_none() {
                continue;
            }

            let command = CustomAction {
                shortcut: shortcut.unwrap().into(),
                command: command.unwrap().into(),
                args: it.map(|s| s.into()).collect(),
            };
            actions.push(command);
        }

        Ok(actions)
    }

    pub fn execute(&self, current_dir: &str) -> ActionResult {
        let mut command = Command::new(&self.command);
        command.current_dir(current_dir);
        for a in &self.args {
            command.arg(a);
        }

        match command.output() {
            Ok(output) => {
                if output.status.success() {
                    ActionResult::from_ok(
                        String::from_utf8_lossy(&output.stdout[..])
                            .into_owned(),
                    )
                } else {
                    let mut out = String::new();
                    out.push_str(
                        &String::from_utf8_lossy(&output.stdout[..])
                            .into_owned()[..],
                    );
                    out.push('\n');
                    out.push('\n');
                    out.push_str(
                        &String::from_utf8_lossy(&output.stderr[..])
                            .into_owned()[..],
                    );
                    ActionResult::from_err(out)
                }
            }
            Err(error) => ActionResult::from_err(error.to_string()),
        }
    }
}

fn next_line<R: BufRead>(reader: &mut R, line: &mut String) -> bool {
    line.clear();
    reader.read_line(line).unwrap_or(0) > 0
}
