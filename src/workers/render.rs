use std::{
    cell::RefCell,
    io::{self},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{spawn, JoinHandle},
};

use anyhow::Result;
use crossbeam::channel::{Receiver, Sender};
use ratatui::{backend::CrosstermBackend, layout::Direction, Terminal, TerminalOptions, Viewport};

use crate::{
    action::{update_contents, window_action},
    context::{Context, Namespace},
    event::Event,
    logger, panic_set_hook,
    ui::WindowEvent,
    window::WindowInit,
};

pub struct Render {
    tx: Sender<Event>,
    rx: Receiver<Event>,
    is_terminated: Arc<AtomicBool>,
    direction: Direction,
}

impl Render {
    pub fn new(
        tx: Sender<Event>,
        rx: Receiver<Event>,
        is_terminated: Arc<AtomicBool>,
        direction: Direction,
    ) -> Self {
        Self {
            direction,
            tx,
            rx,
            is_terminated,
        }
    }

    pub fn start(self) -> JoinHandle<Result<()>> {
        let handle = spawn(move || {
            logger!(info, "Start render worker");

            self.set_panic_hook();

            let is_terminated = self.is_terminated.clone();

            let ret = self.render();

            logger!(info, "Terminated render worker");

            is_terminated.store(true, Ordering::Relaxed);

            ret
        });

        handle
    }

    fn set_panic_hook(&self) {
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
                WindowEvent::Continue => {}
                WindowEvent::CloseWindow => {
                    self.is_terminated
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    // break
                }
                WindowEvent::UpdateContents(ev) => {
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
