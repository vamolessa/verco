use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader},
};

pub struct CustomCommand {
    pub key_chord: String,
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

            let key_chord = it.nth(0);
            if key_chord.is_none() {
                continue;
            }

            let command = it.nth(0);
            if command.is_none() {
                continue;
            }

            let command = CustomCommand {
                key_chord: key_chord.unwrap().into(),
                command: command.unwrap().into(),
                args: it.map(|s| s.into()).collect(),
            };
            commands.push(command);
        }

        Ok(commands)
    }
}

fn next_line<R: BufRead>(reader: &mut R, line: &mut String) -> bool {
    line.clear();
    reader.read_line(line).unwrap_or(0) > 0
}
