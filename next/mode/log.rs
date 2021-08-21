use crate::{
    application::Key,
    backend::LogEntry,
    mode::{
        HeaderInfo, InputStatus, ModeContext, ModeResponse, Output, ReadLine,
        SelectMenu, SelectMenuAction,
    },
    ui::{Draw, Drawer},
};

pub enum Response {
    //
}

#[derive(Default)]
pub struct Mode {
    //
}
impl Mode {
    pub fn on_enter(&mut self, ctx: &ModeContext) {
        //
    }

    pub fn on_key(&mut self, ctx: &ModeContext, key: Key) -> InputStatus {
        InputStatus { pending: false }
    }

    pub fn on_response(&mut self, response: Response) {
        //
    }

    pub fn header(&self) -> HeaderInfo {
        HeaderInfo {
            name: "log",
            waiting_response: false,
        }
    }

    pub fn draw(&self, drawer: &mut Drawer) {
        //
    }
}

