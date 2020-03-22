use crossterm::{
    cursor, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    Result,
};

use std::io::Write;

const HEADER_COLOR: Color = Color::Black;
const ACTION_COLOR: Color = Color::White;
const HEADER_BG_OK_COLOR: Color = Color::Green;
const HEADER_BG_OK_DARK_COLOR: Color = Color::DarkGreen;
const HEADER_BG_ERROR_COLOR: Color = Color::Red;
const HEADER_BG_ERROR_DARK_COLOR: Color = Color::DarkRed;
const HEADER_BG_CANCELED_COLOR: Color = Color::Yellow;
const HEADER_BG_CANCELED_DARK_COLOR: Color = Color::DarkYellow;

#[derive(Clone)]
pub enum HeaderKind {
    Ok,
    Error,
    Canceled,
}

#[derive(Clone)]
pub struct Header<'a> {
    pub kind: HeaderKind,
    pub directory_name: &'a str,
    pub action_name: &'a str,
}

impl<'a> Header<'a> {
    pub fn with_kind(&self, kind: HeaderKind) -> Self {
        Header {
            kind,
            directory_name: self.directory_name,
            action_name: self.action_name,
        }
    }
}

pub fn show_header<W>(write: &mut W, header: &Header) -> Result<u16>
where
    W: Write,
{
    let (w, _) = terminal::size()?;
    let prefix = "Verco @ ";
    let text_size = prefix.len() + header.directory_name.len() + 3 + header.action_name.len();

    let background_color = match header.kind {
        HeaderKind::Ok => HEADER_BG_OK_COLOR,
        HeaderKind::Error => HEADER_BG_ERROR_COLOR,
        HeaderKind::Canceled => HEADER_BG_CANCELED_COLOR,
    };

    let background_dark_color = match header.kind {
        HeaderKind::Ok => HEADER_BG_OK_DARK_COLOR,
        HeaderKind::Error => HEADER_BG_ERROR_DARK_COLOR,
        HeaderKind::Canceled => HEADER_BG_CANCELED_DARK_COLOR,
    };

    queue!(
        write,
        Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        SetBackgroundColor(background_color),
        SetForegroundColor(HEADER_COLOR),
        Print(prefix),
        Print(header.directory_name),
        Print(' '),
        SetBackgroundColor(background_dark_color),
        SetForegroundColor(ACTION_COLOR),
        Print(' '),
        Print(header.action_name),
        Print(' '),
        SetBackgroundColor(background_color),
        SetForegroundColor(HEADER_COLOR),
        Print(" ".repeat(w as usize - text_size)),
        ResetColor,
        Print('\n'),
    )?;

    Ok(text_size as u16)
}
