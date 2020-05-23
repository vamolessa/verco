use crossterm::{
    cursor, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    Result,
};

use std::io::Write;

pub const ENTRY_COLOR: Color = Color::Rgb {
    r: 255,
    g: 180,
    b: 100,
};

const HEADER_COLOR: Color = Color::Black;
const ACTION_COLOR: Color = Color::White;
const HEADER_BG_WAITING_COLOR: Color = Color::Magenta;
const HEADER_BG_WAITING_DARK_COLOR: Color = Color::DarkMagenta;
const HEADER_BG_OK_COLOR: Color = Color::Green;
const HEADER_BG_OK_DARK_COLOR: Color = Color::DarkGreen;
const HEADER_BG_ERROR_COLOR: Color = Color::Red;
const HEADER_BG_ERROR_DARK_COLOR: Color = Color::DarkRed;
const HEADER_BG_CANCELED_COLOR: Color = Color::Yellow;
const HEADER_BG_CANCELED_DARK_COLOR: Color = Color::DarkYellow;

const HEADER_PREFIX: &str = "Verco @ ";
const DIR_NAME_MAX_LENGTH: usize = 32;

pub enum HeaderKind {
    Waiting,
    Ok,
    Error,
    Canceled,
}

pub struct Header<'a> {
    pub action_name: &'a str,
    pub directory_name: &'a str,
}

impl<'a> Header<'a> {
    pub fn full_length(&self) -> usize {
        HEADER_PREFIX.len()
            + self.directory_name.len()
            + 3
            + self.action_name.len()
    }

    pub fn min_length(&self) -> usize {
        HEADER_PREFIX.len()
            + self.directory_name.len().min(DIR_NAME_MAX_LENGTH)
            + 3
            + self.action_name.len()
    }
}

pub fn show_header<W>(
    write: &mut W,
    header: Header,
    kind: HeaderKind,
) -> Result<()>
where
    W: Write,
{
    let background_color = match kind {
        HeaderKind::Waiting => HEADER_BG_WAITING_COLOR,
        HeaderKind::Ok => HEADER_BG_OK_COLOR,
        HeaderKind::Error => HEADER_BG_ERROR_COLOR,
        HeaderKind::Canceled => HEADER_BG_CANCELED_COLOR,
    };

    let background_dark_color = match kind {
        HeaderKind::Waiting => HEADER_BG_WAITING_DARK_COLOR,
        HeaderKind::Ok => HEADER_BG_OK_DARK_COLOR,
        HeaderKind::Error => HEADER_BG_ERROR_DARK_COLOR,
        HeaderKind::Canceled => HEADER_BG_CANCELED_DARK_COLOR,
    };

    let status = match kind {
        HeaderKind::Waiting => "waiting",
        HeaderKind::Ok => "ok",
        HeaderKind::Error => "error",
        HeaderKind::Canceled => "canceled",
    };

    let header_prefix;
    let directory_name;

    let terminal_width = terminal::size()?.0 as usize;
    let mut padding = 0;

    fn fit(
        terminal_width: usize,
        header_length: usize,
        status: &str,
        padding: &mut usize,
    ) -> bool {
        let needed_width = header_length + status.len() + 2;
        if terminal_width >= needed_width {
            *padding = terminal_width - needed_width;
            true
        } else {
            false
        }
    }

    if fit(terminal_width, header.full_length(), status, &mut padding) {
        header_prefix = HEADER_PREFIX;
        directory_name = header.directory_name;
    } else if fit(terminal_width, header.min_length(), status, &mut padding) {
        header_prefix = HEADER_PREFIX;
        directory_name = &header.directory_name
            [(header.directory_name.len() - DIR_NAME_MAX_LENGTH)..];
    } else {
        panic!("window too small");
    }

    queue!(
        write,
        Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        SetBackgroundColor(background_color),
        SetForegroundColor(HEADER_COLOR),
        Print(header_prefix),
        Print(directory_name),
        Print(' '),
        SetBackgroundColor(background_dark_color),
        SetForegroundColor(ACTION_COLOR),
        Print(' '),
        Print(header.action_name),
        Print(' '),
        SetBackgroundColor(background_color),
        SetForegroundColor(HEADER_COLOR),
        Print(" ".repeat(padding)),
        SetBackgroundColor(background_dark_color),
        SetForegroundColor(ACTION_COLOR),
        Print(' '),
        Print(status),
        Print(' '),
        ResetColor,
        cursor::MoveToNextLine(1),
    )
}
