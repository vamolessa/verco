use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader},
    process::Command,
};

pub struct CustomCommand {
    pub shortcut: String,
    pub command: String,
    pub args: Vec<String>,
}

impl CustomCommand {
    pub fn load_custom_commands() -> Vec<CustomCommand> {
        Self::try_load_custom_commands().unwrap_or(Vec::new())
    }

    fn try_load_custom_commands() -> io::Result<Vec<CustomCommand>> {
        let mut path = env::current_dir()?;
        path.push(".verco/custom_commands.txt");
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let mut commands = Vec::new();
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

            let command = CustomCommand {
                shortcut: shortcut.unwrap().into(),
                command: command.unwrap().into(),
                args: it.map(|s| s.into()).collect(),
            };
            commands.push(command);
        }

        Ok(commands)
    }

    pub fn execute(&self, current_dir: &str) -> Result<String, String> {
        let mut command = Command::new(&self.command);
        command.current_dir(current_dir);
        for a in &self.args {
            command.arg(a);
        }

        match command.output() {
            Ok(output) => {
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout[..]).into_owned())
                } else {
                    let mut out = String::new();
                    out.push_str(&String::from_utf8_lossy(&output.stdout[..]).into_owned()[..]);
                    out.push_str("\n\n");
                    out.push_str(&String::from_utf8_lossy(&output.stderr[..]).into_owned()[..]);
                    Err(out)
                }
            }
            Err(error) => Err(error.to_string()),
        }
    }
}

fn next_line<R: BufRead>(reader: &mut R, line: &mut String) -> bool {
    line.clear();
    reader.read_line(line).unwrap_or(0) > 0
}
