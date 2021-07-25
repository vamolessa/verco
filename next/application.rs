use crate::{
    backend::Backend,
    platform::{Context, Key, Keys, PlatformOperation},
};

pub enum Action {
    Status,
}

pub struct Application {
    backend: Box<dyn Backend>,
}
impl Application {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self { backend }
    }

    pub fn update(
        &mut self,
        ctx: &mut Context,
        keys: &mut Keys,
    ) -> Option<PlatformOperation> {
        match keys.next()? {
            Key::Esc => Some(PlatformOperation::Quit),
            Key::Char('s') => self
                .backend
                .status(ctx)
                .map(|_ctx, o| format!("status output:\n{}", o))
                .into_op(Action::Status),
            _ => Some(PlatformOperation::Continue),
        }
    }
}


