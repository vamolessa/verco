use std::io;

pub static ENTER_ALTERNATE_BUFFER_CODE: &[u8] = b"\x1b[?1049h";
pub static EXIT_ALTERNATE_BUFFER_CODE: &[u8] = b"\x1b[?1049l";
pub static HIDE_CURSOR_CODE: &[u8] = b"\x1b[?25l";
pub static SHOW_CURSOR_CODE: &[u8] = b"\x1b[?25h";
pub static RESET_STYLE_CODE: &[u8] = b"\x1b[0;49m";
pub static MODE_256_COLORS_CODE: &[u8] = b"\x1b[=19h";
pub static BEGIN_TITLE_CODE: &[u8] = b"\x1b]0;";
pub static END_TITLE_CODE: &[u8] = b"\x07";

pub struct Color(pub u8, pub u8, pub u8);

pub fn clear_line(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b[2K");
}

pub fn clear_until_new_line(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b[0K");
}

pub fn move_cursor_to(buf: &mut Vec<u8>, x: usize, y: usize) {
    use io::Write;
    let _ = write!(buf, "\x1b[{};{}H", x, y);
}

pub fn move_cursor_to_next_line(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b[1E");
}

pub fn move_cursor_up(buf: &mut Vec<u8>, count: usize) {
    use io::Write;
    let _ = write!(buf, "\x1b[{}A", count);
}

pub fn set_background_color(buf: &mut Vec<u8>, color: Color) {
    use io::Write;
    let _ = write!(buf, "\x1b[48;2;{};{};{}m", color.0, color.1, color.2);
}

pub fn set_foreground_color(buf: &mut Vec<u8>, color: Color) {
    use io::Write;
    let _ = write!(buf, "\x1b[38;2;{};{};{}m", color.0, color.1, color.2);
}

pub fn set_underlined(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b[4m");
}

pub fn set_not_underlined(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\x1b[24m");
}

