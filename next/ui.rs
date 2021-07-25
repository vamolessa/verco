use std::fmt;

pub static ENTER_ALTERNATE_BUFFER_CODE: &str = "\x1b[?1049h";
pub static EXIT_ALTERNATE_BUFFER_CODE: &str = "\x1b[?1049l";
pub static HIDE_CURSOR_CODE: &str = "\x1b[?25l";
pub static SHOW_CURSOR_CODE: &str = "\x1b[?25h";
pub static RESET_STYLE_CODE: &str = "\x1b[0;49m";
pub static MODE_256_COLORS_CODE: &str = "\x1b[=19h";
pub static BEGIN_TITLE_CODE: &str = "\x1b]0;";
pub static END_TITLE_CODE: &str = "\x07";

pub struct Color(pub u8, pub u8, pub u8);

pub fn clear_line(buf: &mut String) {
    buf.push_str("\x1b[2K");
}

pub fn clear_until_new_line(buf: &mut String) {
    buf.push_str("\x1b[0K");
}

pub fn move_cursor_to(buf: &mut String, x: usize, y: usize) {
    use fmt::Write;
    let _ = write!(buf, "\x1b[{};{}H", x, y);
}

pub fn move_cursor_to_next_line(buf: &mut String) {
    buf.push_str("\x1b[1E");
}

pub fn move_cursor_up(buf: &mut String, count: usize) {
    use fmt::Write;
    let _ = write!(buf, "\x1b[{}A", count);
}

pub fn set_background_color(buf: &mut String, color: Color) {
    use fmt::Write;
    let _ = write!(buf, "\x1b[48;2;{};{};{}m", color.0, color.1, color.2);
}

pub fn set_foreground_color(buf: &mut String, color: Color) {
    use fmt::Write;
    let _ = write!(buf, "\x1b[38;2;{};{};{}m", color.0, color.1, color.2);
}

pub fn set_underlined(buf: &mut String) {
    buf.push_str("\x1b[4m");
}

pub fn set_not_underlined(buf: &mut String) {
    buf.push_str("\x1b[24m");
}

