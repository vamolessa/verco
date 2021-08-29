#[cfg(windows)]
use winapi::{
    shared::{
        minwindef::{BOOL, DWORD, FALSE, TRUE},
        ntdef::NULL,
        winerror::{
            ERROR_IO_PENDING, ERROR_MORE_DATA, ERROR_PIPE_CONNECTED,
            WAIT_TIMEOUT,
        },
    },
    um::{
        consoleapi::{
            GetConsoleMode, ReadConsoleInputW, SetConsoleCtrlHandler,
            SetConsoleMode,
        },
        errhandlingapi::GetLastError,
        fileapi::{
            CreateFileW, FindClose, FindFirstFileW, GetFileType, ReadFile,
            WriteFile, OPEN_EXISTING,
        },
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        ioapiset::GetOverlappedResult,
        minwinbase::OVERLAPPED,
        namedpipeapi::{
            ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe,
            SetNamedPipeHandleState,
        },
        processenv::{GetCommandLineW, GetStdHandle},
        processthreadsapi::{
            CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW,
        },
        stringapiset::{MultiByteToWideChar, WideCharToMultiByte},
        synchapi::{CreateEventW, SetEvent, WaitForMultipleObjects},
        winbase::{
            GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock,
            FILE_FLAG_OVERLAPPED, FILE_TYPE_CHAR, GMEM_MOVEABLE, INFINITE,
            NORMAL_PRIORITY_CLASS, PIPE_ACCESS_DUPLEX, PIPE_READMODE_BYTE,
            PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, STARTF_USESTDHANDLES,
            STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
            WAIT_OBJECT_0,
        },
        wincon::{
            GetConsoleScreenBufferInfo, ENABLE_PROCESSED_OUTPUT,
            ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_WINDOW_INPUT,
        },
        wincontypes::{
            INPUT_RECORD, KEY_EVENT, LEFT_ALT_PRESSED, LEFT_CTRL_PRESSED,
            RIGHT_ALT_PRESSED, RIGHT_CTRL_PRESSED, WINDOW_BUFFER_SIZE_EVENT,
        },
        winnls::CP_UTF8,
        winnt::{GENERIC_READ, GENERIC_WRITE, HANDLE, MAXIMUM_WAIT_OBJECTS},
        winuser::{
            CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard,
            SetClipboardData, CF_UNICODETEXT, VK_BACK, VK_DELETE, VK_DOWN,
            VK_END, VK_ESCAPE, VK_F1, VK_F24, VK_HOME, VK_LEFT, VK_NEXT,
            VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
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

pub enum PlatformEvent {
    Key(Key),
    Resize(u16, u16),
}

pub struct Platform;

// ========================================================= UNIX

#[cfg(unix)]
pub struct PlatformInitGuard;

#[cfg(unix)]
impl Platform {
    pub fn is_pipped() -> bool {
        todo!();
    }

    pub fn init() -> PlatformInitGuard {
        todo!();
        PlatformInitGuard
    }

    pub fn next_terminal_event() -> PlatformEvent {
        todo!()
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
    input_handle_original_mode: Option<DWORD>,
    output_handle_original_mode: Option<DWORD>,
}

#[cfg(windows)]
impl Platform {
    pub fn is_pipped() -> bool {
        match Self::get_std_handle(STD_INPUT_HANDLE) {
            Some(input_handle) => unsafe {
                GetFileType(input_handle) != FILE_TYPE_CHAR
            },
            None => true,
        }
    }

    pub fn init() -> PlatformInitGuard {
        let input_handle = Self::get_std_handle(STD_INPUT_HANDLE);
        let output_handle = Self::get_std_handle(STD_OUTPUT_HANDLE);

        let input_handle_original_mode = input_handle
            .map(|h| Self::swap_console_mode(h, ENABLE_WINDOW_INPUT));
        let output_handle_original_mode = output_handle.map(|h| {
            Self::swap_console_mode(
                h,
                ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
            )
        });

        PlatformInitGuard {
            input_handle_original_mode,
            output_handle_original_mode,
        }
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

    pub fn next_terminal_event() -> PlatformEvent {
        todo!()
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
        let input_handle = Platform::get_std_handle(STD_INPUT_HANDLE);
        let output_handle = Platform::get_std_handle(STD_OUTPUT_HANDLE);

        if let Some((handle, mode)) =
            input_handle.zip(self.input_handle_original_mode)
        {
            Platform::set_console_mode(handle, mode);
        }
        if let Some((handle, mode)) =
            output_handle.zip(self.output_handle_original_mode)
        {
            Platform::set_console_mode(handle, mode);
        }
    }
}

