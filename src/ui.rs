use std::fmt;

use crate::mode::{Filter, Output, ReadLine, SelectMenu};

pub const HEADER_LINE_COUNT: usize = 2;
pub const RESERVED_LINES_COUNT: usize = HEADER_LINE_COUNT + 1;

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
    DarkGray,
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
            Self::DarkGray => "8",
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

    pub fn header(
        &mut self,
        current_mode_name: &str,
        left_help: &str,
        right_help: &str,
        spinner: u8,
    ) {
        const ALL_MODES: &[(&str, u8)] = &[
            ("status", b's'),
            ("log", b'l'),
            ("branches", b'b'),
            ("tags", b't'),
        ];
        fn mode_tabs_len(tabs: &[(&str, u8)]) -> usize {
            let mut len = 0;
            for (name, _) in tabs {
                len += "[x]".len() + name.len() + 1;
            }
            len
        }

        let background_color = Color::Black;
        let foreground_color = Color::DarkYellow;

        move_cursor_to_zero(&mut self.buf);

        set_background_color(&mut self.buf, background_color);
        set_foreground_color(&mut self.buf, foreground_color);
        self.buf.push(b' ');
        self.buf.push(spinner);
        self.buf.push(b' ');

        set_background_color(&mut self.buf, foreground_color);
        set_foreground_color(&mut self.buf, background_color);
        self.buf.push(b' ');
        self.buf.extend_from_slice(current_mode_name.as_bytes());
        self.buf.push(b' ');

        set_background_color(&mut self.buf, background_color);
        set_foreground_color(&mut self.buf, foreground_color);

        let (modes_before, modes_after) =
            match ALL_MODES.iter().position(|&(m, _)| m == current_mode_name) {
                Some(i) => (&ALL_MODES[..i], &ALL_MODES[i + 1..]),
                None => (ALL_MODES, &[][..]),
            };
        let modes_before_len = mode_tabs_len(modes_before);
        let modes_after_len = mode_tabs_len(modes_after);
        let current_mode_len = 3 + 1 + current_mode_name.len() + 1;

        let spacer_len = (self.viewport_size.0 as usize)
            .saturating_sub(modes_before_len + modes_after_len + current_mode_len);
        self.buf.extend(std::iter::repeat(b' ').take(spacer_len));

        for &(mode_name, shortcut) in modes_before.iter().chain(modes_after) {
            self.buf.push(b'[');
            self.buf.push(shortcut);
            self.buf.push(b']');
            self.buf.extend_from_slice(mode_name.as_bytes());
            self.buf.push(b' ');
        }

        clear_until_new_line(&mut self.buf);
        move_cursor_to_next_line(&mut self.buf);

        set_background_color(&mut self.buf, foreground_color);
        set_foreground_color(&mut self.buf, background_color);

        let mut left_help = left_help.as_bytes();
        let mut right_help = right_help.as_bytes();

        let available_width = self.viewport_size.0.saturating_sub(1) as usize;
        if left_help.len() > available_width {
            left_help = &left_help[..available_width];
            right_help = &[];
        } else if left_help.len() + right_help.len() > available_width {
            let overflow_len = left_help.len() + right_help.len() - available_width;
            right_help = &right_help[..right_help.len() - overflow_len];
        }

        let spacer_len = 1 + available_width - left_help.len() - right_help.len();
        self.buf.extend_from_slice(left_help);
        self.buf.extend(std::iter::repeat(b' ').take(spacer_len));
        self.buf.extend_from_slice(right_help);

        move_cursor_to_next_line(&mut self.buf);

        set_background_color(&mut self.buf, Color::Black);
        set_foreground_color(&mut self.buf, Color::White);
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
            if line_count + 1 >= self.viewport_size.1 as _ {
                break;
            }
        }

        line_count
    }

    pub fn readline(&mut self, readline: &ReadLine, placeholder: &str) {
        let input = readline.input();

        set_background_color(&mut self.buf, Color::Black);
        set_foreground_color(&mut self.buf, Color::White);
        self.buf.extend_from_slice(input.as_bytes());

        set_background_color(&mut self.buf, Color::DarkRed);
        self.buf.push(b' ');
        set_background_color(&mut self.buf, Color::Black);

        if input.is_empty() {
            set_foreground_color(&mut self.buf, Color::DarkGray);
            self.buf.extend_from_slice(placeholder.as_bytes());
        }
    }

    pub fn filter(&mut self, filter: &Filter) -> usize {
        let text = filter.as_str();
        if !filter.is_filtering() {
            return 0;
        }

        const PREFIX: &str = "filter:";
        set_background_color(&mut self.buf, Color::DarkRed);
        set_foreground_color(&mut self.buf, Color::White);
        self.buf.extend_from_slice(PREFIX.as_bytes());

        let available_width = (self.viewport_size.0 as usize).saturating_sub(PREFIX.len() + 2);
        let (trimmed, text) = match text.char_indices().rev().nth(available_width) {
            Some((i, _)) => (true, &text[i..]),
            None => (false, text),
        };
        self.buf.extend_from_slice(text.as_bytes());

        if filter.has_focus() {
            set_background_color(&mut self.buf, Color::White);
            self.buf.push(b' ');
            if !trimmed {
                set_background_color(&mut self.buf, Color::DarkRed);
            }
        }

        self.next_line();
        1
    }

    pub fn select_menu<'entries, I, E>(
        &mut self,
        select: &SelectMenu,
        header_height: usize,
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
            (self.viewport_size.1 as usize).saturating_sub(RESERVED_LINES_COUNT + header_height);

        for (i, entry) in entries.enumerate().skip(select.scroll()) {
            let hovered = i == cursor_index;
            if hovered {
                set_background_color(&mut self.buf, Color::DarkMagenta);
            }

            line_count += entry.draw(self, hovered, hovered && show_full_hovered_entry);

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

