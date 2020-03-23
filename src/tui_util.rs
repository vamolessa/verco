use crossterm::{
    cursor, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    Result,
};

use std::io::Write;

pub const HEADER_COLOR: Color = Color::Black;
pub const ACTION_COLOR: Color = Color::White;
pub const HEADER_BG_OK_COLOR: Color = Color::Green;
pub const HEADER_BG_OK_DARK_COLOR: Color = Color::DarkGreen;
pub const HEADER_BG_ERROR_COLOR: Color = Color::Red;
pub const HEADER_BG_ERROR_DARK_COLOR: Color = Color::DarkRed;
pub const HEADER_BG_CANCELED_COLOR: Color = Color::Yellow;
pub const HEADER_BG_CANCELED_DARK_COLOR: Color = Color::DarkYellow;

const HEADER_PREFIX: &str = "Verco @ ";

pub enum HeaderKind {
    Ok,
    Error,
    Canceled,
}

pub struct Header<'a> {
    pub action_name: &'a str,
    pub directory_name: String,
}

impl<'a> Header<'a> {
    pub fn length(&self) -> usize {
        HEADER_PREFIX.len() + self.directory_name.len() + 3 + self.action_name.len()
    }
}

pub fn show_header<W>(write: &mut W, header: &Header, kind: HeaderKind) -> Result<()>
where
    W: Write,
{
    let background_color = match kind {
        HeaderKind::Ok => HEADER_BG_OK_COLOR,
        HeaderKind::Error => HEADER_BG_ERROR_COLOR,
        HeaderKind::Canceled => HEADER_BG_CANCELED_COLOR,
    };

    let background_dark_color = match kind {
        HeaderKind::Ok => HEADER_BG_OK_DARK_COLOR,
        HeaderKind::Error => HEADER_BG_ERROR_DARK_COLOR,
        HeaderKind::Canceled => HEADER_BG_CANCELED_DARK_COLOR,
    };

    let status = match kind {
        HeaderKind::Ok => "ok",
        HeaderKind::Error => "error",
        HeaderKind::Canceled => "canceled",
    };

    queue!(
        write,
        Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        SetBackgroundColor(background_color),
        SetForegroundColor(HEADER_COLOR),
        Print(HEADER_PREFIX),
        Print(&header.directory_name),
        Print(' '),
        SetBackgroundColor(background_dark_color),
        SetForegroundColor(ACTION_COLOR),
        Print(' '),
        Print(header.action_name),
        Print(' '),
        SetBackgroundColor(background_color),
        SetForegroundColor(HEADER_COLOR),
        Print(" ".repeat(terminal::size()?.0 as usize - header.length() - status.len() - 2)),
        SetBackgroundColor(background_dark_color),
        SetForegroundColor(ACTION_COLOR),
        Print(' '),
        Print(status),
        Print(' '),
        ResetColor,
        Print('\n'),
    )
}
