use crossterm::event::{self, Event, KeyCode, KeyEvent};

pub fn read_char() -> crossterm::Result<char> {
    loop {
        if let Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            ..
        }) = event::read()?
        {
            return Ok(c);
        }
    }
}

pub fn read_line() -> crossterm::Result<String> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let len = line.trim_end_matches(&['\r', '\n'][..]).len();
    line.truncate(len);
    Ok(line)
}
