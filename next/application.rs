use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
};

use crossterm::{event, terminal};

use crate::{backend::Backend, controller};

static VIEWPORT_WIDTH: AtomicUsize = AtomicUsize::new(0);
static VIEWPORT_HEIGHT: AtomicUsize = AtomicUsize::new(0);

static CURRENT_ACTION_KIND: AtomicUsize =
    AtomicUsize::new(ActionKind::Help as _);

fn resize(width: u16, height: u16) {
    VIEWPORT_WIDTH.store(width as _, Ordering::Relaxed);
    VIEWPORT_HEIGHT.store(height as _, Ordering::Relaxed);
}

#[derive(Clone, Copy)]
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
    pub fn from_key_event(ev: event::KeyEvent) -> Self {
        match ev.code {
            event::KeyCode::Backspace => Self::Backspace,
            event::KeyCode::Enter => Self::Enter,
            event::KeyCode::Left => Self::Left,
            event::KeyCode::Right => Self::Right,
            event::KeyCode::Up => Self::Up,
            event::KeyCode::Down => Self::Down,
            event::KeyCode::Home => Self::Home,
            event::KeyCode::End => Self::End,
            event::KeyCode::PageUp => Self::PageUp,
            event::KeyCode::PageDown => Self::PageDown,
            event::KeyCode::Tab => Self::Tab,
            event::KeyCode::BackTab => Self::BackTab,
            event::KeyCode::Delete => Self::Delete,
            event::KeyCode::Insert => Self::Insert,
            event::KeyCode::F(_) => Self::None,
            event::KeyCode::Char(mut c) => {
                if ev.modifiers & event::KeyModifiers::ALT
                    != event::KeyModifiers::NONE
                {
                    return Self::None;
                }

                if ev.modifiers & event::KeyModifiers::SHIFT
                    != event::KeyModifiers::NONE
                {
                    c = c.to_ascii_uppercase();
                }
                if ev.modifiers & event::KeyModifiers::CONTROL
                    != event::KeyModifiers::NONE
                {
                    Self::Ctrl(c)
                } else {
                    Self::Char(c)
                }
            }
            event::KeyCode::Null => Self::None,
            event::KeyCode::Esc => Self::Esc,
        }
    }
}

pub enum ApplicationEvent {
    Key(Key),
    Redraw,
}
impl ApplicationEvent {
    pub fn next() -> Self {
        loop {
            match event::read().unwrap() {
                event::Event::Key(key) => {
                    return Self::Key(Key::from_key_event(key));
                }
                event::Event::Mouse(_) => (),
                event::Event::Resize(width, height) => {
                    resize(width, height);
                    return Self::Redraw;
                }
            }
        }
    }
}

pub enum ApplicationFlow {
    Continue,
    Pending,
    Quit,
}

#[derive(Clone, Copy)]
pub enum ActionKind {
    Help,
    Status,
    LEN,
}
impl ActionKind {
    pub fn current() -> usize {
        CURRENT_ACTION_KIND.load(Ordering::Relaxed)
    }

    pub fn set_as_current(self) {
        CURRENT_ACTION_KIND.store(self as _, Ordering::Relaxed)
    }
}

enum ActionState {
    Waiting,
    Ok,
    Err,
}

struct ActionOutput {
    text: String,
    state: ActionState,
}
impl Default for ActionOutput {
    fn default() -> Self {
        Self {
            text: String::new(),
            state: ActionState::Ok,
        }
    }
}

pub struct Application {
    backend: Arc<dyn Backend>,
    outputs: Arc<[Mutex<ActionOutput>; ActionKind::LEN as _]>,
    previous_key: Key,
}
impl Application {
    pub fn previous_key(&self) -> Key {
        self.previous_key
    }

    pub fn schedule(
        &self,
        action: ActionKind,
        f: fn(&dyn Backend) -> Result<String, String>,
    ) {
        action.set_as_current();

        let mut output = self.outputs[action as usize].lock().unwrap();
        if let ActionState::Waiting = output.state {
            return;
        }
        output.state = ActionState::Waiting;

        let backend = self.backend.clone();
        let outputs = self.outputs.clone();

        thread::spawn(move || {
            use std::ops::Deref;
            let result = f(backend.deref());

            let mut output = outputs[action as usize].lock().unwrap();
            match result {
                Ok(text) => {
                    output.state = ActionState::Ok;
                    output.text = text;
                }
                Err(text) => {
                    output.state = ActionState::Err;
                    output.text = text;
                }
            }

            if ActionKind::current() == action as _ {
                println!("output:\n{}", &output.text);
            }
        });
    }

    pub fn redraw(&self) {
        let output = self.outputs[ActionKind::current()].lock().unwrap();
        println!("redraw:\n{}", &output.text);
    }
}

pub fn run(backend: Arc<dyn Backend>) {
    match terminal::size() {
        Ok((width, height)) => resize(width, height),
        Err(_) => return,
    };

    let mut app = Application {
        backend,
        outputs: Default::default(),
        previous_key: Key::None,
    };

    loop {
        match ApplicationEvent::next() {
            ApplicationEvent::Key(key) => {
                match controller::handle_key(&app, key) {
                    ApplicationFlow::Continue => app.previous_key = Key::None,
                    ApplicationFlow::Pending => app.previous_key = key,
                    ApplicationFlow::Quit => break,
                }
            }
            ApplicationEvent::Redraw => (),
        }

        app.redraw()
    }
}

