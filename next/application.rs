use std::{env, fs, io, panic, path::Path};

use crate::{
    platform::{
        Key, PlatformEvent, PlatformRequest, ProcessHandle, ProcessTag,
    },
    ui,
};

pub struct AnyError;
impl<T> From<T> for AnyError
where
    T: std::error::Error,
{
    fn from(_: T) -> Self {
        Self
    }
}

pub struct Application {
    stdout: io::StdoutLock<'static>,
}
impl Application {
    pub fn new() -> Self {
        let stdout = Box::new(io::stdout());
        let stdout = Box::leak(stdout);
        let mut stdout = stdout.lock();

        use io::Write;
        let _ = stdout.write_all(ui::ENTER_ALTERNATE_BUFFER_CODE);
        let _ = stdout.write_all(ui::HIDE_CURSOR_CODE);
        let _ = stdout.write_all(ui::MODE_256_COLORS_CODE);
        stdout.flush().unwrap();

        Self { stdout }
    }

    pub fn update(
        &mut self,
        events: &[PlatformEvent],
        requests: &mut Vec<PlatformRequest>,
    ) -> bool {
        true
    }
}
impl Drop for Application {
    fn drop(&mut self) {
        use io::Write;
        let _ = self.stdout.write_all(ui::EXIT_ALTERNATE_BUFFER_CODE);
        let _ = self.stdout.write_all(ui::SHOW_CURSOR_CODE);
        let _ = self.stdout.write_all(ui::RESET_STYLE_CODE);
        let _ = self.stdout.flush();
    }
}

