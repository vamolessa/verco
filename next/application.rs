use crate::{
    backend::Backend,
    platform::{Context, Key, Keys, PlatformOperation},
};

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
                .map(|_ctx, o| {
                    println!("status output:\n{}", o);
                })
                .into(),
            _ => Some(PlatformOperation::Continue),
        }
    }
}

