mod action;
mod window;

use std::{
    cell::RefCell,
    io::{self},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use ratatui::{backend::CrosstermBackend, layout::Direction, Terminal, TerminalOptions, Viewport};

use crate::{
    kube::context::{Context, Namespace},
    logger,
    message::Message,
    panic_set_hook,
    theme::Theme,
    ui::WindowAction,
};

use self::{
    action::{update_contents, window_action},
    window::WindowInit,
};

pub struct Render {
    tx: Sender<Message>,
    rx: Receiver<Message>,
    is_terminated: Arc<AtomicBool>,
    direction: Direction,
    theme: Theme,
}

impl Render {
    pub fn new(
        tx: Sender<Message>,
        rx: Receiver<Message>,
        is_terminated: Arc<AtomicBool>,
        direction: Direction,
        theme: Theme,
    ) -> Self {
        Self {
            direction,
            tx,
            rx,
            is_terminated,
            theme,
        }
    }

    pub fn start(self) -> Result<()> {
        logger!(info, "render start");

        let ret = self.render();

        self.is_terminated.store(true, Ordering::Relaxed);

        logger!(info, "render end");

        ret
    }

    pub fn set_panic_hook(&self) {
        let is_terminated = self.is_terminated.clone();

        panic_set_hook!({
            is_terminated.store(true, Ordering::Relaxed);
        });
    }

    fn render(&self) -> Result<()> {
        let namespace = Rc::new(RefCell::new(Namespace::new()));
        let context = Rc::new(RefCell::new(Context::new()));

        let mut window = WindowInit::new(
            self.direction,
            self.tx.clone(),
            context.clone(),
            namespace.clone(),
            self.theme.clone(),
        )
        .build();

        let mut terminal = Terminal::with_options(
            CrosstermBackend::new(io::stdout()),
            TerminalOptions {
                viewport: Viewport::Fullscreen,
            },
        )?;

        terminal.clear()?;

        while !self.is_terminated.load(Ordering::Relaxed) {
            terminal.draw(|f| {
                window.render(f);
            })?;

            match window_action(&mut window, &self.rx) {
                WindowAction::Continue => {}
                WindowAction::CloseWindow => {
                    self.is_terminated
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    // break
                }
                WindowAction::UpdateContents(ev) => {
                    update_contents(
                        &mut window,
                        ev,
                        &mut context.borrow_mut(),
                        &mut namespace.borrow_mut(),
                    );
                }
            }
        }

        Ok(())
    }
}
