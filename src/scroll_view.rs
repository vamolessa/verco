use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    QueueableCommand, Result,
};

use std::io::Write;

use crate::{ctrlc_handler::CtrlcHandler, input};

const HEADER_COLOR: Color = Color::White;
const OK_BG_COLOR: Color = Color::Green;
const ERR_BG_COLOR: Color = Color::Red;

pub fn show_scroll_view<W>(
    write: &mut W,
    ctrlc_handler: &mut CtrlcHandler,
    content: std::result::Result<String, String>,
) -> Result<()>
where
    W: Write,
{
    let terminal_size = terminal::size()?;
    execute!(
        write,
        Clear(ClearType::FromCursorDown),
        SetBackgroundColor(if content.is_ok() {
            OK_BG_COLOR
        } else {
            ERR_BG_COLOR
        }),
        SetForegroundColor(HEADER_COLOR),
        Print(" ".repeat(terminal_size.0 as usize - 1)),
        ResetColor,
        Print('\n')
    )?;

    let cursor_position = cursor::position()?;
    let width = (terminal_size.0 - 1) as usize;
    let height = (terminal_size.1 - cursor_position.1) as usize;

    let content = match content {
        Ok(text) => text,
        Err(error) => error,
    };

    let content_height = content.chars().filter(|c| *c == '\n').count() + 1;

    for line in content.lines().take(height) {
        if let Some((last_index, _)) = line.char_indices().take_while(|(i, _)| *i < width).last() {
            write.queue(Print(&line[..last_index]))?;
        }
        write.queue(Print('\n'))?;
    }

    write.flush()?;

    Ok(())
}
