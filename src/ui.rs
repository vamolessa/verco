use std::fmt;

use crate::mode::{Output, ReadLine, SelectMenu};

pub static ENTER_ALTERNATE_BUFFER_CODE: &[u8] = b"\x1b[?1049h";
pub static EXIT_ALTERNATE_BUFFER_CODE: &[u8] = b"\x1b[?1049l";
pub static HIDE_CURSOR_CODE: &[u8] = b"\x1b[?25l";
pub static SHOW_CURSOR_CODE: &[u8] = b"\x1b[?25h";
pub static RESET_STYLE_CODE: &[u8] = b"\x1b[0;49m";
pub static BEGIN_TITLE_CODE: &[u8] = b"\x1b]0;";
pub static END_TITLE_CODE: &[u8] = b"\x07";

pub fn clear_until_new_line(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b[0K");
}

pub fn clear_to_end(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b[0J");
}

pub fn move_cursor_to_zero(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b[0;0H");
}

pub fn move_cursor_to_next_line(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b[1E");
}

pub fn set_background_color(buf: &mut Vec<u8>, color: Color) {
    buf.extend_from_slice(b"\x1b[48;5;");
    buf.extend_from_slice(color.code().as_bytes());
    buf.push(b'm');
}

static BEGIN_FOREGROUND_COLOR_CODE: &str = "\x1b[38;5;";
pub fn set_foreground_color(buf: &mut Vec<u8>, color: Color) {
    buf.extend_from_slice(BEGIN_FOREGROUND_COLOR_CODE.as_bytes());
    buf.extend_from_slice(color.code().as_bytes());
    buf.push(b'm');
}

#[derive(Clone, Copy)]
pub enum Color {
    Black,
    DarkRed,
    DarkGreen,
    DarkYellow,
    DarkBlue,
    DarkMagenta,
    White,
}
impl Color {
    fn code(&self) -> &str {
        match self {
            Self::Black => "0",
            Self::DarkRed => "1",
            Self::DarkGreen => "2",
            Self::DarkYellow => "3",
            Self::DarkBlue => "4",
            Self::DarkMagenta => "5",
            Self::White => "15",
        }
    }
}
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(BEGIN_FOREGROUND_COLOR_CODE)?;
        f.write_str(self.code())?;
        f.write_str("m")?;
        Ok(())
    }
}

pub trait SelectEntryDraw {
    fn draw(&self, drawer: &mut Drawer, hovered: bool, full: bool) -> usize;
}

pub struct Drawer {
    buf: Vec<u8>,
    pub viewport_size: (u16, u16),
}

impl Drawer {
    pub fn new(mut buf: Vec<u8>, viewport_size: (u16, u16)) -> Self {
        buf.clear();
        Self { buf, viewport_size }
    }

    pub fn take_buf(self) -> Vec<u8> {
        self.buf
    }

    pub fn clear_to_bottom(&mut self) {
        set_background_color(&mut self.buf, Color::Black);
        clear_to_end(&mut self.buf);
    }

    pub fn header(&mut self, header: &str, spinner: u8) {
        let background_color = Color::DarkYellow;
        let foreground_color = Color::Black;

        move_cursor_to_zero(&mut self.buf);
        set_background_color(&mut self.buf, background_color);
        set_foreground_color(&mut self.buf, foreground_color);
        self.buf.extend_from_slice(&[b' ', spinner, b' ']);
        set_background_color(&mut self.buf, foreground_color);
        set_foreground_color(&mut self.buf, background_color);
        self.buf.push(b' ');
        self.buf.extend_from_slice(header.as_bytes());
        self.buf.push(b' ');

        let size = crate::platform::Platform::terminal_size();
        use std::io::Write;
        write!(self.buf, " {:?} {:?}", self.viewport_size, size).unwrap();

        set_background_color(&mut self.buf, background_color);
        clear_until_new_line(&mut self.buf);
        move_cursor_to_next_line(&mut self.buf);
        self.buf.extend_from_slice(RESET_STYLE_CODE);
    }

    pub fn str(&mut self, line: &str) {
        self.buf.extend_from_slice(line.as_bytes());
    }

    pub fn fmt(&mut self, args: fmt::Arguments) {
        use std::io::Write;
        self.buf.write_fmt(args).unwrap();
    }

    pub fn next_line(&mut self) {
        clear_until_new_line(&mut self.buf);
        move_cursor_to_next_line(&mut self.buf);
    }

    pub fn output(&mut self, output: &Output) -> usize {
        let tab_bytes = [b' '; 4];
        let mut utf8_buf = [0; 4];

        let mut line_count = 0;
        for line in output.lines_from_scroll() {
            let mut x = 0;
            for c in line.chars() {
                match c {
                    '\t' => {
                        self.buf.extend_from_slice(&tab_bytes);
                        x += tab_bytes.len();
                    }
                    _ => {
                        let bytes = c.encode_utf8(&mut utf8_buf).as_bytes();
                        self.buf.extend_from_slice(bytes);
                        x += 1;
                    }
                }

                if x >= self.viewport_size.0 as _ {
                    x -= self.viewport_size.0 as usize;
                    line_count += 1;
                }
            }

            self.next_line();

            line_count += 1;
            if line_count >= self.viewport_size.1 as _ {
                break;
            }
        }

        line_count
    }

    pub fn readline(&mut self, readline: &ReadLine) {
        set_background_color(&mut self.buf, Color::Black);
        set_foreground_color(&mut self.buf, Color::White);
        self.buf.extend_from_slice(readline.input().as_bytes());
        set_background_color(&mut self.buf, Color::DarkRed);
        self.buf.push(b' ');
        set_background_color(&mut self.buf, Color::Black);
    }

    pub fn select_menu<'entries, I, E>(
        &mut self,
        select: &SelectMenu,
        header_height: u16,
        show_full_hovered_entry: bool,
        entries: I,
    ) where
        I: 'entries + Iterator<Item = &'entries E>,
        E: 'entries + SelectEntryDraw,
    {
        let cursor_index = select.cursor();

        set_background_color(&mut self.buf, Color::Black);
        set_foreground_color(&mut self.buf, Color::White);

        let mut line_count = 0;
        let max_line_count =
            self.viewport_size.1.saturating_sub(1 + header_height) as usize;

        for (i, entry) in entries.enumerate().skip(select.scroll()) {
            let hovered = i == cursor_index;
            if hovered {
                set_background_color(&mut self.buf, Color::DarkMagenta);
            }

            line_count +=
                entry.draw(self, hovered, hovered && show_full_hovered_entry);

            clear_until_new_line(&mut self.buf);
            move_cursor_to_next_line(&mut self.buf);

            if hovered {
                set_background_color(&mut self.buf, Color::Black);
            }

            if line_count >= max_line_count {
                break;
            }
        }
    }
}

