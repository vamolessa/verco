#[cfg(unix)]
use std::os::unix::io::RawFd;

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

// ========================================================= UNIX

#[cfg(unix)]
pub struct Platform {
    original: libc::termios,
}

#[cfg(unix)]
impl Platform {
    pub fn new() -> Option<(Self, PlatformEventReader)> {
        let is_pipped = unsafe { libc::isatty(libc::STDIN_FILENO) == 0 };
        if is_pipped {
            return None;
        }

        let original = unsafe {
            let mut original = std::mem::zeroed();
            libc::tcgetattr(libc::STDIN_FILENO, &mut original);
            let mut new = original.clone();
            new.c_iflag &= !(libc::IGNBRK
                | libc::BRKINT
                | libc::PARMRK
                | libc::ISTRIP
                | libc::INLCR
                | libc::IGNCR
                | libc::ICRNL
                | libc::IXON);
            new.c_oflag &= !libc::OPOST;
            new.c_cflag &= !(libc::CSIZE | libc::PARENB);
            new.c_cflag |= libc::CS8;
            new.c_lflag &=
                !(libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN);
            new.c_lflag |= libc::NOFLSH;
            new.c_cc[libc::VMIN] = 0;
            new.c_cc[libc::VTIME] = 0;
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &new);
            original
        };
        let backspace_code = original.c_cc[libc::VERASE];

        Some((Self { original }, PlatformEventReader::new(backspace_code)))
    }

    pub fn terminal_size() -> (u16, u16) {
        let mut size: libc::winsize = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::ioctl(
                libc::STDOUT_FILENO,
                libc::TIOCGWINSZ as _,
                &mut size as *mut libc::winsize,
            )
        };
        if result == -1 || size.ws_col == 0 {
            panic!("could not get terminal size");
        }

        (size.ws_col as _, size.ws_row as _)
    }
}

#[cfg(unix)]
impl Drop for Platform {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &self.original)
        };
    }
}

#[cfg(unix)]
pub struct PlatformEventReader {
    backspace_code: u8,
    buf: Vec<u8>,
    queue_fd: RawFd,
    resize_signal_fd: Option<RawFd>,
}

const MAX_TRIGGERED_EVENT_COUNT: usize = 32;

#[cfg(unix)]
impl PlatformEventReader {
    // ========================================================= LINUX

    #[cfg(target_os = "linux")]
    pub fn new(backspace_code: u8) -> Self {
        let queue_fd = unsafe { libc::epoll_create1(0) };
        if queue_fd == -1 {
            panic!("could not create epoll");
        }

        let resize_signal_fd = unsafe {
            let mut signals = std::mem::zeroed();
            let result = libc::sigemptyset(&mut signals);
            if result == -1 {
                panic!("could not create signal fd");
            }
            let result = libc::sigaddset(&mut signals, libc::SIGWINCH);
            if result == -1 {
                panic!("could not create signal fd");
            }
            let result = libc::sigprocmask(
                libc::SIG_BLOCK,
                &signals,
                std::ptr::null_mut(),
            );
            if result == -1 {
                panic!("could not create signal fd");
            }
            let fd = libc::signalfd(-1, &signals, 0);
            if fd == -1 {
                panic!("could not create signal fd");
            }
            fd
        };

        fn epoll_add_fd(epoll_fd: RawFd, fd: RawFd, index: usize) {
            let mut event = libc::epoll_event {
                events: (libc::EPOLLIN
                    | libc::EPOLLERR
                    | libc::EPOLLRDHUP
                    | libc::EPOLLHUP) as _,
                u64: index as _,
            };
            let result = unsafe {
                libc::epoll_ctl(epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut event)
            };
            if result == -1 {
                panic!("could not add event");
            }
        }

        epoll_add_fd(queue_fd, libc::STDIN_FILENO, 0);
        epoll_add_fd(queue_fd, resize_signal_fd, 1);

        let resize_signal_fd = Some(resize_signal_fd);
        let mut buf = Vec::with_capacity(1024);
        let capacity = buf.capacity();
        buf.resize(capacity, 0);

        Self {
            backspace_code,
            buf,
            queue_fd,
            resize_signal_fd,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn read_terminal_events(
        &mut self,
        keys: &mut Vec<Key>,
        resize: &mut Option<(u16, u16)>,
    ) {
        fn epoll_wait<'a>(
            epoll_fd: RawFd,
            events: &'a mut [libc::epoll_event],
        ) -> impl 'a + ExactSizeIterator<Item = usize> {
            let timeout = -1;
            let mut len = unsafe {
                libc::epoll_wait(
                    epoll_fd,
                    events.as_mut_ptr(),
                    events.len() as _,
                    timeout,
                )
            };
            if len == -1 {
                if PlatformEventReader::errno() == libc::EINTR {
                    len = 0;
                } else {
                    panic!("could not wait for events");
                }
            }

            events[..len as usize].iter().map(|e| e.u64 as _)
        }

        const DEFAULT_EVENT: libc::epoll_event =
            libc::epoll_event { events: 0, u64: 0 };
        let mut epoll_events = [DEFAULT_EVENT; MAX_TRIGGERED_EVENT_COUNT];

        for event_index in epoll_wait(self.queue_fd, &mut epoll_events) {
            match event_index {
                0 => match Self::read(libc::STDIN_FILENO, &mut self.buf) {
                    Ok(0) | Err(()) => continue,
                    Ok(len) => Self::parse_terminal_keys(
                        &self.buf[..len],
                        self.backspace_code,
                        keys,
                    ),
                },
                1 => {
                    if let Some(fd) = self.resize_signal_fd {
                        let mut buf =
                            [0; std::mem::size_of::<libc::signalfd_siginfo>()];
                        if Self::read(fd, &mut buf) != Ok(buf.len()) {
                            panic!("could not read from signal fd");
                        }
                        *resize = Some(Platform::terminal_size());
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    // ========================================================= BSD

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
    ))]
    pub fn new(backspace_code: u8) -> Self {
        let queue_fd = unsafe { libc::kqueue() };
        if queue_fd == -1 {
            panic!("could not create kqueue");
        }

        let stdin_event = libc::kevent {
            ident: libc::STDIN_FILENO as _,
            filter: libc::EVFILT_READ,
            flags: libc::EV_ADD,
            fflags: 0,
            data: 0,
            udata: 0 as _,
        };
        let resize_event = libc::kevent {
            ident: libc::SIGWINCH as _,
            filter: libc::EVFILT_SIGNAL,
            flags: libc::EV_ADD,
            fflags: 0,
            data: 0,
            udata: 1 as _,
        };

        fn modify_kqueue(kqueue_fd: RawFd, event: &libc::kevent) -> bool {
            let result = unsafe {
                libc::kevent(
                    kqueue_fd,
                    event as _,
                    1,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            };
            result == 0
        }

        if !modify_kqueue(queue_fd, &stdin_event) {
            panic!("could not add event");
        }
        if !modify_kqueue(queue_fd, &resize_event) {
            panic!("could not add event");
        }

        Self {
            backspace_code,
            buf: Vec::with_capacity(1024),
            queue_fd,
            resize_signal_fd: None,
        }
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
    ))]
    pub fn read_terminal_events(
        &mut self,
        keys: &mut Vec<Key>,
        resize: &mut Option<(u16, u16)>,
    ) {
        struct TriggeredEvent {
            pub index: usize,
            pub data: isize,
        }

        pub fn kqueue_wait<'a>(
            kqueue_fd: RawFd,
            events: &'a mut [libc::kevent],
        ) -> impl 'a + ExactSizeIterator<Item = Result<TriggeredEvent, ()>>
        {
            let timeout = std::ptr::null();
            let mut len = unsafe {
                libc::kevent(
                    kqueue_fd,
                    [].as_ptr(),
                    0,
                    events.as_mut_ptr(),
                    events.len() as _,
                    timeout,
                )
            };
            if len == -1 {
                if PlatformEventReader::errno() == libc::EINTR {
                    len = 0;
                } else {
                    panic!("could not wait for events");
                }
            }

            events[..len as usize].iter().map(|e| {
                if e.flags & libc::EV_ERROR != 0 {
                    Err(())
                } else {
                    Ok(TriggeredEvent {
                        index: e.udata as _,
                        data: e.data as _,
                    })
                }
            })
        }

        const DEFAULT_KEVENT: libc::kevent = libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        let mut kqueue_events = [DEFAULT_KEVENT; MAX_TRIGGERED_EVENT_COUNT];

        for event in kqueue_wait(self.queue_fd, &mut kqueue_events) {
            match event {
                Ok(TriggeredEvent { index: 0, data }) => {
                    self.buf.resize(data as _, 0);
                    match read(libc::STDIN_FILENO, &mut buf) {
                        Ok(0) | Err(()) => continue,
                        Ok(len) => Self::parse_terminal_keys(
                            &self.buf[..len],
                            self.backspace_code,
                            keys,
                        ),
                    }
                }
                Ok(TriggeredEvent { index: 1, .. }) => {
                    *resize = Some(Platform::terminal_size())
                }
                Ok(_) => unreachable!(),
                Err(()) => break,
            }
        }
    }

    // ========================================================= COMMON

    pub fn errno() -> libc::c_int {
        #[cfg(target_os = "linux")]
        unsafe {
            *libc::__errno_location()
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly",
        ))]
        unsafe {
            *libc::__error()
        }
    }

    pub fn read(fd: RawFd, buf: &mut [u8]) -> Result<usize, ()> {
        let len =
            unsafe { libc::read(fd, buf.as_mut_ptr() as _, buf.len() as _) };
        if len >= 0 {
            Ok(len as _)
        } else {
            Err(())
        }
    }

    fn parse_terminal_keys(
        mut buf: &[u8],
        backspace_code: u8,
        keys: &mut Vec<Key>,
    ) {
        loop {
            let (key, rest) = match buf {
                &[] => break,
                &[b, ref rest @ ..] if b == backspace_code => {
                    (Key::Backspace, rest)
                }
                &[0x1b, b'[', b'5', b'~', ref rest @ ..] => (Key::PageUp, rest),
                &[0x1b, b'[', b'6', b'~', ref rest @ ..] => {
                    (Key::PageDown, rest)
                }
                &[0x1b, b'[', b'A', ref rest @ ..] => (Key::Up, rest),
                &[0x1b, b'[', b'B', ref rest @ ..] => (Key::Down, rest),
                &[0x1b, b'[', b'C', ref rest @ ..] => (Key::Right, rest),
                &[0x1b, b'[', b'D', ref rest @ ..] => (Key::Left, rest),
                &[0x1b, b'[', b'1', b'~', ref rest @ ..]
                | &[0x1b, b'[', b'7', b'~', ref rest @ ..]
                | &[0x1b, b'[', b'H', ref rest @ ..]
                | &[0x1b, b'O', b'H', ref rest @ ..] => (Key::Home, rest),
                &[0x1b, b'[', b'4', b'~', ref rest @ ..]
                | &[0x1b, b'[', b'8', b'~', ref rest @ ..]
                | &[0x1b, b'[', b'F', ref rest @ ..]
                | &[0x1b, b'O', b'F', ref rest @ ..] => (Key::End, rest),
                &[0x1b, b'[', b'3', b'~', ref rest @ ..] => (Key::Delete, rest),
                &[0x1b, ref rest @ ..] => (Key::Esc, rest),
                &[0x8, ref rest @ ..] => (Key::Backspace, rest),
                &[b'\r', ref rest @ ..] => (Key::Enter, rest),
                &[b'\t', ref rest @ ..] => (Key::Tab, rest),
                &[0x7f, ref rest @ ..] => (Key::Delete, rest),
                &[b @ 0b0..=0b11111, ref rest @ ..] => {
                    let byte = b | 0b01100000;
                    (Key::Ctrl(byte as _), rest)
                }
                _ => match buf
                    .iter()
                    .position(|b| b.is_ascii())
                    .unwrap_or(buf.len())
                {
                    0 => (Key::Char(buf[0] as _), &buf[1..]),
                    len => {
                        let (c, rest) = buf.split_at(len);
                        match std::str::from_utf8(c) {
                            Ok(s) => match s.chars().next() {
                                Some(c) => (Key::Char(c), rest),
                                None => (Key::None, rest),
                            },
                            Err(_) => (Key::None, rest),
                        }
                    }
                },
            };
            buf = rest;
            keys.push(key);
        }
    }
}
#[cfg(unix)]
impl Drop for PlatformEventReader {
    fn drop(&mut self) {
        unsafe { libc::close(self.queue_fd) };
    }
}

// ========================================================= WINDOWS

#[cfg(windows)]
pub struct Platform {
    input_handle_original_mode: DWORD,
    output_handle_original_mode: DWORD,
}

#[cfg(windows)]
impl Platform {
    pub fn new() -> Option<(Self, PlatformEventReader)> {
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

        Some((
            Self {
                input_handle_original_mode,
                output_handle_original_mode,
            },
            PlatformEventReader,
        ))
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
impl Drop for Platform {
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

#[cfg(windows)]
pub struct PlatformEventReader;

#[cfg(windows)]
impl PlatformEventReader {
    pub fn read_terminal_events(
        &mut self,
        keys: &mut Vec<Key>,
        resize: &mut Option<(u16, u16)>,
    ) {
        let input_handle = match Platform::get_std_handle(STD_INPUT_HANDLE) {
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
}

