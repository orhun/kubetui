pub mod input;
pub mod tick;

pub mod kubernetes;

mod util;

use crate::kubernetes::Kube;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};

#[derive(PartialEq, Clone, Copy)]
pub enum UserEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

impl From<char> for UserEvent {
    fn from(c: char) -> Self {
        UserEvent::Key(KeyEvent::from(KeyCode::Char(c)))
    }
}

pub enum Event {
    Kube(Kube),
    User(UserEvent),
    Tick,
}
