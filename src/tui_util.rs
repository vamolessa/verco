use std::io::Write;

use crossterm::{
    cursor, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    Result,
};

pub const SELECTED_BG_COLOR: Color = Color::Rgb {
    r: 80,
    g: 80,
    b: 80,
};
pub const ENTRY_COLOR: Color = Color::Rgb {
    r: 255,
    g: 180,
    b: 100,
};

pub const LOG_COLORS: &[Color] = &[
    Color::White,
    Color::Rgb {
        r: 211,
        g: 153,
        b: 33,
    },
    Color::Rgb {
        r: 52,
        g: 113,
        b: 134,
    },
    Color::Rgb {
        r: 137,
        g: 151,
        b: 29,
    },
    Color::Rgb {
        r: 251,
        g: 73,
        b: 47,
    },
    Color::White,
];

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
    terminal_size: TerminalSize,
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

    let terminal_width = terminal_size.width as usize;
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

#[derive(Default, Clone, Copy)]
pub struct TerminalSize {
    pub width: u16,
    pub height: u16,
}

impl TerminalSize {
    pub fn get() -> Result<Self> {
        let size = terminal::size()?;
        Ok(Self {
            width: size.0,
            height: size.1,
        })
    }
}

#[derive(Clone, Copy)]
pub struct AvailableSize {
    pub width: usize,
    pub height: usize,
}

impl AvailableSize {
    pub fn from_temrinal_size(terminal_size: TerminalSize) -> Self {
        Self {
            width: terminal_size.width as usize,
            height: terminal_size.height as usize - 2,
        }
    }
}

pub fn move_cursor(
    scroll: &mut usize,
    cursor: &mut usize,
    available_size: AvailableSize,
    entry_count: usize,
    delta: i32,
) {
    if entry_count == 0 {
        *scroll = 0;
        *cursor = 0;
        return;
    }

    let previous_cursor = *cursor;
    let target_cursor = *cursor as i32 + delta;
    *cursor = if target_cursor < 0 {
        if previous_cursor == 0 {
            (target_cursor + entry_count as i32) as usize % entry_count
        } else {
            0
        }
    } else if target_cursor >= entry_count as i32 {
        if previous_cursor == entry_count - 1 {
            (target_cursor + entry_count as i32) as usize % entry_count
        } else {
            entry_count - 1
        }
    } else {
        target_cursor as usize
    };

    if cursor < scroll {
        *scroll = *cursor;
    } else if *cursor >= *scroll + available_size.height - 1 {
        *scroll = 1 + *cursor - available_size.height;
    }
}
