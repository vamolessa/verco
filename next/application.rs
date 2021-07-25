use crate::{
    backend::Backend,
    platform::{Context, Key, Keys},
};

pub struct Application {
    backend: Box<dyn Backend>,
    //
}
impl Application {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self { backend }
    }

    pub fn update(
        &mut self,
        ctx: &mut Context,
        keys: &mut Keys,
    ) -> Option<bool> {
        match keys.next()? {
            Key::Esc => Some(false),
            //Key::Char('s') => self.backend.status(),
            _ => Some(true),
        }
    }
}

