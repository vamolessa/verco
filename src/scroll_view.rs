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

pub fn show_scroll_view<W>(
    write: &mut W,
    ctrlc_handler: &mut CtrlcHandler,
    content: &str,
) -> Result<()>
where
    W: Write,
{
    write.queue(cursor::MoveTo(0, 1))?;

    let terminal_size = terminal::size()?;
    let width = terminal_size.0 as usize;
    let height = terminal_size.1 as usize - 1;

    // let content_height = content.lines().count() + 1;
    for line in content.lines().take(height) {
        if let Some((last_index, _)) = line.char_indices().take_while(|(i, _)| *i < width).last() {
            write.queue(Print(&line[..last_index + 1]))?;
        }
        write.queue(Print('\n'))?;
    }

    write.flush()?;
    Ok(())
}
