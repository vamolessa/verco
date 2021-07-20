use std::{io, os::windows::io::IntoRawHandle, process::Child, time::Duration};

use winapi::{
    shared::{
        minwindef::{BOOL, DWORD, FALSE, TRUE},
        ntdef::NULL,
        winerror::{ERROR_IO_PENDING, ERROR_MORE_DATA, WAIT_TIMEOUT},
    },
    um::{
        consoleapi::{
            GetConsoleMode, ReadConsoleInputW, SetConsoleCtrlHandler,
            SetConsoleMode,
        },
        errhandlingapi::GetLastError,
        fileapi::{GetFileType, ReadFile},
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        ioapiset::GetOverlappedResult,
        minwinbase::OVERLAPPED,
        processenv::GetStdHandle,
        synchapi::{CreateEventW, SetEvent, WaitForMultipleObjects},
        winbase::{
            FILE_TYPE_CHAR, INFINITE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
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
        winnt::{HANDLE, MAXIMUM_WAIT_OBJECTS},
        winuser::{
            VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F24,
            VK_HOME, VK_LEFT, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SPACE,
            VK_TAB, VK_UP,
        },
    },
};

use crate::{
    application::{Application, ProcessTag},
    platform::{Key, PlatformEvent, PlatformRequest, ProcessHandle},
};

const CONSOLE_EVENT_BUFFER_LEN: usize = 32;

pub fn main() {
    let input_handle = match get_std_handle(STD_INPUT_HANDLE) {
        Some(handle) => handle,
        None => return,
    };
    let output_handle = match get_std_handle(STD_OUTPUT_HANDLE) {
        Some(handle) => handle,
        None => return,
    };

    if is_pipped(&input_handle) {
        return;
    }

    set_ctrlc_handler();

    let console_input_mode = ConsoleMode::new(&input_handle);
    console_input_mode.set(ENABLE_WINDOW_INPUT);
    let console_output_mode = ConsoleMode::new(&output_handle);
    console_output_mode
        .set(ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING);

    let mut console_event_buf =
        [unsafe { std::mem::zeroed() }; CONSOLE_EVENT_BUFFER_LEN];

    let mut wait_handles = [std::ptr::null_mut(); MAXIMUM_WAIT_OBJECTS as _];
    wait_handles[0] = input_handle.0;

    let mut application = match Application::new() {
        Some(application) => application,
        None => return,
    };

    let mut events = Vec::new();
    let size = get_console_size(&output_handle);
    events.push(PlatformEvent::Resize(size.0 as _, size.1 as _));

    let mut processes: Vec<AsyncProcess> = Vec::new();

    let mut timeout = Some(Duration::ZERO);
    loop {
        let mut wait_handles_len = 1;
        for process in &processes {
            if wait_handles_len == MAXIMUM_WAIT_OBJECTS as _ {
                break;
            }

            wait_handles[wait_handles_len] = process.stdout.event().handle();
            wait_handles_len += 1;
        }

        match wait_for_multiple_objects(
            &wait_handles[..wait_handles_len],
            timeout,
        ) {
            Some(i) => {
                match i {
                    0 => {
                        let console_events = read_console_input(
                            &input_handle,
                            &mut console_event_buf,
                        );
                        parse_console_events(console_events, &mut events);
                    }
                    i => {
                        let index = i - 1;
                        let process = &mut processes[index];
                        let tag = process.tag;
                        match process.stdout.read_async() {
                            Ok(None) => (),
                            Ok(Some(buf)) if !buf.is_empty() => {
                                events.push(PlatformEvent::ProcessOutput {
                                    tag,
                                    buf,
                                })
                            }
                            _ => {
                                process.kill();
                                processes.remove(index);
                                events.push(PlatformEvent::ProcessExit { tag });
                            }
                        }
                    }
                }
                timeout = Some(Duration::ZERO);
            }
            None => {
                if !application.update(&events) {
                    break;
                }
                events.clear();

                for request in application.drain_platform_requests() {
                    match request {
                        PlatformRequest::SpawnProcess {
                            tag,
                            mut command,
                            buf_len,
                        } => match command.spawn() {
                            Ok(child) => {
                                let handle =
                                    ProcessHandle(processes.len() as _);
                                match AsyncProcess::new(child, tag, buf_len) {
                                    Some(process) => {
                                        events.push(
                                            PlatformEvent::ProcessSpawned {
                                                tag,
                                                handle,
                                            },
                                        );
                                        processes.push(process);
                                    }
                                    None => events.push(
                                        PlatformEvent::ProcessExit { tag },
                                    ),
                                };
                            }
                            Err(_) => {
                                events.push(PlatformEvent::ProcessExit { tag });
                            }
                        },
                        PlatformRequest::WriteToProcess { handle, buf } => {
                            let process = &mut processes[handle.0 as usize];
                            if !process.write(&buf) {
                                let tag = process.tag;
                                process.kill();
                                processes.remove(handle.0 as _);
                                events.push(PlatformEvent::ProcessExit { tag });
                            }
                        }
                        PlatformRequest::CloseProcessInput { handle } => {
                            processes[handle.0 as usize].close_input();
                        }
                        PlatformRequest::KillProcess { handle } => {
                            let process = &mut processes[handle.0 as usize];
                            let tag = process.tag;
                            process.kill();
                            processes.remove(handle.0 as _);
                            events.push(PlatformEvent::ProcessExit { tag });
                        }
                    }
                }

                timeout = None;
            }
        }
    }

    drop(console_input_mode);
    drop(console_output_mode);
}

fn get_last_error() -> DWORD {
    unsafe { GetLastError() }
}

fn set_ctrlc_handler() {
    unsafe extern "system" fn handler(_ctrl_type: DWORD) -> BOOL {
        FALSE
    }

    if unsafe { SetConsoleCtrlHandler(Some(handler), TRUE) } == FALSE {
        panic!("could not set ctrl handler");
    }
}

fn get_std_handle(which: DWORD) -> Option<Handle> {
    let handle = unsafe { GetStdHandle(which) };
    if handle != NULL && handle != INVALID_HANDLE_VALUE {
        Some(Handle(handle))
    } else {
        None
    }
}

fn get_console_size(output_handle: &Handle) -> (usize, usize) {
    let mut console_info = unsafe { std::mem::zeroed() };
    let result = unsafe {
        GetConsoleScreenBufferInfo(output_handle.0, &mut console_info)
    };
    if result == FALSE {
        panic!("could not get console size");
    }
    (console_info.dwSize.X as _, console_info.dwSize.Y as _)
}

fn read_console_input<'a>(
    input_handle: &Handle,
    events: &'a mut [INPUT_RECORD],
) -> &'a [INPUT_RECORD] {
    let mut event_count: DWORD = 0;
    let result = unsafe {
        ReadConsoleInputW(
            input_handle.0,
            events.as_mut_ptr(),
            events.len() as _,
            &mut event_count,
        )
    };
    if result == FALSE {
        panic!("could not read console events");
    }
    &events[..(event_count as usize)]
}

enum ReadResult {
    Waiting,
    Ok(usize),
    Err,
}

struct AsyncReader {
    handle: Handle,
    event: Event,
    overlapped: Overlapped,
    pending_io: bool,
}
impl AsyncReader {
    pub fn new(handle: Handle) -> Self {
        let event = Event::manual();
        event.notify();
        let overlapped = Overlapped::with_event(&event);

        Self {
            handle,
            event,
            overlapped,
            pending_io: false,
        }
    }

    pub fn event(&self) -> &Event {
        &self.event
    }

    pub fn read_async(&mut self, buf: &mut [u8]) -> ReadResult {
        let mut read_len = 0;
        if self.pending_io {
            let result = unsafe {
                GetOverlappedResult(
                    self.handle.0,
                    self.overlapped.as_mut_ptr(),
                    &mut read_len,
                    FALSE,
                )
            };

            self.pending_io = false;

            if result == FALSE {
                match get_last_error() {
                    ERROR_MORE_DATA => {
                        self.event.notify();
                        ReadResult::Ok(read_len as _)
                    }
                    _ => ReadResult::Err,
                }
            } else {
                self.event.notify();
                ReadResult::Ok(read_len as _)
            }
        } else {
            let result = unsafe {
                ReadFile(
                    self.handle.0,
                    buf.as_mut_ptr() as _,
                    buf.len() as _,
                    &mut read_len,
                    self.overlapped.as_mut_ptr(),
                )
            };

            if result == FALSE {
                match get_last_error() {
                    ERROR_IO_PENDING => {
                        self.pending_io = true;
                        ReadResult::Waiting
                    }
                    _ => ReadResult::Err,
                }
            } else {
                self.event.notify();
                ReadResult::Ok(read_len as _)
            }
        }
    }
}

fn is_pipped(handle: &Handle) -> bool {
    unsafe { GetFileType(handle.0) != FILE_TYPE_CHAR }
}

fn wait_for_multiple_objects(
    handles: &[HANDLE],
    timeout: Option<Duration>,
) -> Option<usize> {
    let timeout = match timeout {
        Some(duration) => duration.as_millis() as _,
        None => INFINITE,
    };
    let len = MAXIMUM_WAIT_OBJECTS.min(handles.len() as DWORD);
    let result = unsafe {
        WaitForMultipleObjects(len, handles.as_ptr(), FALSE, timeout)
    };
    if result == WAIT_TIMEOUT {
        None
    } else if result >= WAIT_OBJECT_0 && result < (WAIT_OBJECT_0 + len) {
        Some((result - WAIT_OBJECT_0) as _)
    } else {
        panic!("could not wait for event")
    }
}

struct Handle(pub HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

fn create_event(manual_reset: bool, initial_state: bool) -> HANDLE {
    let manual_reset = if manual_reset { TRUE } else { FALSE };
    let initial_state = if initial_state { TRUE } else { FALSE };
    let handle = unsafe {
        CreateEventW(
            std::ptr::null_mut(),
            manual_reset,
            initial_state,
            std::ptr::null(),
        )
    };
    if handle == NULL {
        panic!("could not create event");
    }
    handle
}

fn set_event(handle: HANDLE) {
    if unsafe { SetEvent(handle) } == FALSE {
        panic!("could not set event");
    }
}

struct Event(HANDLE);
impl Event {
    pub fn manual() -> Self {
        Self(create_event(true, false))
    }

    pub fn handle(&self) -> HANDLE {
        self.0
    }

    pub fn notify(&self) {
        set_event(self.0);
    }
}
impl Drop for Event {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

struct Overlapped(OVERLAPPED);
impl Overlapped {
    pub fn with_event(event: &Event) -> Self {
        let mut overlapped = unsafe { std::mem::zeroed::<OVERLAPPED>() };
        overlapped.hEvent = event.handle();
        Self(overlapped)
    }

    pub fn as_mut_ptr(&mut self) -> *mut OVERLAPPED {
        &mut self.0
    }
}

struct ConsoleMode {
    console_handle: HANDLE,
    original_mode: DWORD,
}
impl ConsoleMode {
    pub fn new(console_handle: &Handle) -> Self {
        let console_handle = console_handle.0;
        let mut original_mode = DWORD::default();
        let result =
            unsafe { GetConsoleMode(console_handle, &mut original_mode) };
        if result == FALSE {
            panic!("could not get console mode");
        }
        Self {
            console_handle,
            original_mode,
        }
    }

    pub fn set(&self, mode: DWORD) {
        let result = unsafe { SetConsoleMode(self.console_handle, mode) };
        if result == FALSE {
            panic!("could not set console mode");
        }
    }
}
impl Drop for ConsoleMode {
    fn drop(&mut self) {
        self.set(self.original_mode);
    }
}

struct ProcessPipe {
    reader: AsyncReader,
    buf_len: usize,
    current_buf: Option<Vec<u8>>,
}
impl ProcessPipe {
    pub fn new(reader: AsyncReader, buf_len: usize) -> Self {
        reader.event.notify();

        Self {
            reader,
            buf_len,
            current_buf: None,
        }
    }

    pub fn event(&self) -> &Event {
        self.reader.event()
    }

    pub fn read_async(&mut self) -> Result<Option<Vec<u8>>, ()> {
        let mut buf = match self.current_buf.take() {
            Some(buf) => buf,
            None => Vec::with_capacity(self.buf_len),
        };
        buf.resize(self.buf_len, 0);

        match self.reader.read_async(&mut buf) {
            ReadResult::Waiting => {
                self.current_buf = Some(buf);
                Ok(None)
            }
            ReadResult::Ok(len) => {
                buf.truncate(len);
                Ok(Some(buf))
            }
            ReadResult::Err => Err(()),
        }
    }
}

struct AsyncProcess {
    alive: bool,
    child: Child,
    tag: ProcessTag,
    pub stdout: ProcessPipe,
}
impl AsyncProcess {
    pub fn new(
        mut child: Child,
        tag: ProcessTag,
        buf_len: usize,
    ) -> Option<Self> {
        let stdout = child
            .stdout
            .take()
            .map(IntoRawHandle::into_raw_handle)
            .map(|h| {
                let reader = AsyncReader::new(Handle(h as _));
                ProcessPipe::new(reader, buf_len)
            })?;

        Some(Self {
            alive: true,
            child,
            tag,
            stdout,
        })
    }

    pub fn write(&mut self, buf: &[u8]) -> bool {
        use io::Write;
        match self.child.stdin {
            Some(ref mut stdin) => stdin.write_all(buf).is_ok(),
            None => true,
        }
    }

    pub fn close_input(&mut self) {
        self.child.stdin = None;
    }

    pub fn kill(&mut self) {
        if !self.alive {
            return;
        }

        self.alive = false;
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Drop for AsyncProcess {
    fn drop(&mut self) {
        self.kill();
        self.alive = false;
    }
}

fn parse_console_events(
    console_events: &[INPUT_RECORD],
    events: &mut Vec<PlatformEvent>,
) {
    for event in console_events {
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
                    VK_F1..=VK_F24 => Key::F((keycode - VK_F1 + 1) as _),
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
                            let c = (keycode - CHAR_A) as u8 + b'a';
                            Key::Alt(c.to_ascii_lowercase() as _)
                        } else if control_key_state & CTRL_PRESSED_MASK != 0 {
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
                    events.push(PlatformEvent::Key(key));
                }
            }
            WINDOW_BUFFER_SIZE_EVENT => {
                let size =
                    unsafe { event.Event.WindowBufferSizeEvent().dwSize };
                events.push(PlatformEvent::Resize(size.X as _, size.Y as _));
            }
            _ => (),
        }
    }
}

