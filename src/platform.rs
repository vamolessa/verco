#[cfg(windows)]
use winapi::{
    shared::{
        minwindef::{DWORD, FALSE},
        ntdef::NULL,
    },
    um::{
        consoleapi::{GetConsoleMode, ReadConsoleInputW, SetConsoleMode},
        fileapi::GetFileType,
        handleapi::INVALID_HANDLE_VALUE,
        processenv::GetStdHandle,
        winbase::{FILE_TYPE_CHAR, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE},
        wincon::{
            GetConsoleScreenBufferInfo, ENABLE_PROCESSED_OUTPUT,
            ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_WINDOW_INPUT,
        },
        wincontypes::{
            KEY_EVENT, LEFT_ALT_PRESSED, LEFT_CTRL_PRESSED, RIGHT_ALT_PRESSED,
            RIGHT_CTRL_PRESSED, WINDOW_BUFFER_SIZE_EVENT,
        },
        winnt::HANDLE,
        winuser::{
            VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F24,
            VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SPACE,
            VK_TAB, VK_UP,
        },
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Key {
    None,
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Delete,
    Insert,
    Char(char),
    Ctrl(char),
    Esc,
}
impl Key {
    pub fn is_submit(&self) -> bool {
        matches!(self, Self::Enter | Self::Char('\n') | Self::Ctrl('m'))
    }

    pub fn is_cancel(&self) -> bool {
        matches!(self, Self::Esc | Self::Ctrl('c'))
    }
}

pub struct Platform;

// ========================================================= UNIX

#[cfg(unix)]
pub struct PlatformInitGuard;

#[cfg(unix)]
impl Platform {
    pub fn init() -> Option<PlatformInitGuard> {
        todo!();
        None
    }

    pub fn terminal_size() -> (u16, u16) {
        todo!();
    }

    pub fn read_terminal_events(
        keys: &mut Vec<Key>,
        resize: &mut Option<(u16, u16)>,
    ) {
        todo!();
    }
}
#[cfg(unix)]
impl Drop for PlatformInitGuard {
    fn drop(&mut self) {
        todo!();
    }
}

// ========================================================= WINDOWS

#[cfg(windows)]
pub struct PlatformInitGuard {
    input_handle_original_mode: DWORD,
    output_handle_original_mode: DWORD,
}

#[cfg(windows)]
impl Platform {
    pub fn init() -> Option<PlatformInitGuard> {
        let input_handle = Self::get_std_handle(STD_INPUT_HANDLE)?;
        let output_handle = Self::get_std_handle(STD_OUTPUT_HANDLE)?;

        let is_pipped = unsafe { GetFileType(input_handle) != FILE_TYPE_CHAR };
        if is_pipped {
            return None;
        }

        let input_handle_original_mode =
            Self::swap_console_mode(input_handle, ENABLE_WINDOW_INPUT);
        let output_handle_original_mode = Self::swap_console_mode(
            output_handle,
            ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
        );

        Some(PlatformInitGuard {
            input_handle_original_mode,
            output_handle_original_mode,
        })
    }

    pub fn terminal_size() -> (u16, u16) {
        let output_handle = match Self::get_std_handle(STD_OUTPUT_HANDLE) {
            Some(handle) => handle,
            None => panic!("could not get console size"),
        };

        let mut console_info = unsafe { std::mem::zeroed() };
        let result = unsafe {
            GetConsoleScreenBufferInfo(output_handle, &mut console_info)
        };
        if result == FALSE {
            panic!("could not get console size");
        }
        (console_info.dwSize.X as _, console_info.dwSize.Y as _)
    }

    pub fn read_terminal_events(
        keys: &mut Vec<Key>,
        resize: &mut Option<(u16, u16)>,
    ) {
        let input_handle = match Self::get_std_handle(STD_INPUT_HANDLE) {
            Some(handle) => handle,
            None => return,
        };

        let mut events = [unsafe { std::mem::zeroed() }; 32];
        let mut event_count = 0;
        let result = unsafe {
            ReadConsoleInputW(
                input_handle,
                events.as_mut_ptr(),
                events.len() as _,
                &mut event_count,
            )
        };
        if result == FALSE {
            panic!("could not read console events");
        }

        for event in &events[..(event_count as usize)] {
            match event.EventType {
                KEY_EVENT => {
                    let event = unsafe { event.Event.KeyEvent() };
                    if event.bKeyDown == FALSE {
                        continue;
                    }

                    let control_key_state = event.dwControlKeyState;
                    let keycode = event.wVirtualKeyCode as i32;
                    let unicode_char = unsafe { *event.uChar.UnicodeChar() };
                    let repeat_count = event.wRepeatCount as usize;

                    const CHAR_A: i32 = b'A' as _;
                    const CHAR_Z: i32 = b'Z' as _;
                    let key = match keycode {
                        VK_BACK => Key::Backspace,
                        VK_RETURN => Key::Enter,
                        VK_LEFT => Key::Left,
                        VK_RIGHT => Key::Right,
                        VK_UP => Key::Up,
                        VK_DOWN => Key::Down,
                        VK_HOME => Key::Home,
                        VK_END => Key::End,
                        VK_PRIOR => Key::PageUp,
                        VK_NEXT => Key::PageDown,
                        VK_TAB => Key::Tab,
                        VK_DELETE => Key::Delete,
                        VK_F1..=VK_F24 => continue,
                        VK_ESCAPE => Key::Esc,
                        VK_SPACE => {
                            match std::char::decode_utf16(std::iter::once(
                                unicode_char,
                            ))
                            .next()
                            {
                                Some(Ok(c)) => Key::Char(c),
                                _ => continue,
                            }
                        }
                        CHAR_A..=CHAR_Z => {
                            const ALT_PRESSED_MASK: DWORD =
                                LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED;
                            const CTRL_PRESSED_MASK: DWORD =
                                LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED;

                            if control_key_state & ALT_PRESSED_MASK != 0 {
                                continue;
                            } else if control_key_state & CTRL_PRESSED_MASK != 0
                            {
                                let c = (keycode - CHAR_A) as u8 + b'a';
                                Key::Ctrl(c.to_ascii_lowercase() as _)
                            } else {
                                match std::char::decode_utf16(std::iter::once(
                                    unicode_char,
                                ))
                                .next()
                                {
                                    Some(Ok(c)) => Key::Char(c),
                                    _ => continue,
                                }
                            }
                        }
                        _ => match std::char::decode_utf16(std::iter::once(
                            unicode_char,
                        ))
                        .next()
                        {
                            Some(Ok(c)) if c.is_ascii_graphic() => Key::Char(c),
                            _ => continue,
                        },
                    };

                    for _ in 0..repeat_count {
                        keys.push(key);
                    }
                }
                WINDOW_BUFFER_SIZE_EVENT => {
                    let size =
                        unsafe { event.Event.WindowBufferSizeEvent().dwSize };
                    *resize = Some((size.X as _, size.Y as _));
                }
                _ => (),
            }
        }
    }

    fn get_std_handle(which: DWORD) -> Option<HANDLE> {
        let handle = unsafe { GetStdHandle(which) };
        if handle != NULL && handle != INVALID_HANDLE_VALUE {
            Some(handle)
        } else {
            None
        }
    }

    fn set_console_mode(handle: HANDLE, mode: DWORD) {
        let result = unsafe { SetConsoleMode(handle, mode) };
        if result == FALSE {
            panic!("could not set console mode");
        }
    }

    fn swap_console_mode(handle: HANDLE, new_mode: DWORD) -> DWORD {
        let mut original_mode = 0;
        let result = unsafe { GetConsoleMode(handle, &mut original_mode) };
        if result == FALSE {
            panic!("could not get console mode");
        }
        Self::set_console_mode(handle, new_mode);
        original_mode
    }
}
#[cfg(windows)]
impl Drop for PlatformInitGuard {
    fn drop(&mut self) {
        if let Some(handle) = Platform::get_std_handle(STD_INPUT_HANDLE) {
            Platform::set_console_mode(handle, self.input_handle_original_mode);
        }
        if let Some(handle) = Platform::get_std_handle(STD_OUTPUT_HANDLE) {
            Platform::set_console_mode(
                handle,
                self.output_handle_original_mode,
            );
        }
    }
}

