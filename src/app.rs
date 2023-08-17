use crate::{
    action::{update_contents, window_action},
    command::Command,
    context::{Context, Namespace},
    event::{input::read_key, kubernetes::KubeWorker, tick::tick, Event},
    logging::Logger,
    ui::WindowEvent,
    window::WindowInit,
};
use anyhow::Result;
use clap::Parser;
use crossbeam::channel::{bounded, Receiver, Sender};
use ratatui::{prelude::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use std::{
    cell::RefCell,
    io,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread, time,
};

#[macro_export]
macro_rules! enable_raw_mode {
    () => {
        crossterm::terminal::enable_raw_mode().expect("failed to enable raw mode");
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture,
            crossterm::event::EnableFocusChange
        )
        .expect("failed to enable raw mode");
    };
}

#[macro_export]
macro_rules! disable_raw_mode {
    () => {
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
            crossterm::event::DisableFocusChange,
            crossterm::cursor::Show
        )
        .expect("failed to restore terminal");
        crossterm::terminal::disable_raw_mode().expect("failed to disable raw mode");
    };
}

pub struct App {
    command: Command,
}

impl App {
    pub fn init() -> Result<Self> {
        let command = Command::parse();

        if command.logging {
            Logger::init()?;
        }

        Ok(App { command })
    }

    pub fn run(self) -> Result<()> {
        let split_mode = self.command.split_mode();
        let kube_worker_config = self.command.kube_worker_config();

        let (tx_input, rx_main): (Sender<Event>, Receiver<Event>) = bounded(128);
        let (tx_main, rx_kube): (Sender<Event>, Receiver<Event>) = bounded(256);
        let tx_kube = tx_input.clone();
        let tx_tick = tx_input.clone();

        let is_terminated = Arc::new(AtomicBool::new(false));

        let is_terminated_clone = is_terminated.clone();

        let read_key_handler = thread::spawn(move || read_key(tx_input, is_terminated_clone));

        let is_terminated_clone = is_terminated.clone();
        let kube_process_handler = thread::spawn(move || {
            KubeWorker::new(tx_kube, rx_kube, is_terminated_clone, kube_worker_config).run()
        });

        let is_terminated_clone = is_terminated.clone();
        let tick_handler = thread::spawn(move || {
            tick(
                tx_tick,
                time::Duration::from_millis(200),
                is_terminated_clone,
            )
        });

        let backend = CrosstermBackend::new(io::stdout());

        let namespace = Rc::new(RefCell::new(Namespace::new()));
        let context = Rc::new(RefCell::new(Context::new()));

        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fullscreen,
            },
        )?;

        let mut window =
            WindowInit::new(split_mode, tx_main, context.clone(), namespace.clone()).build();

        terminal.clear()?;

        while !is_terminated.load(Ordering::Relaxed) {
            terminal.draw(|f| {
                window.render(f);
            })?;

            match window_action(&mut window, &rx_main) {
                WindowEvent::Continue => {}
                WindowEvent::CloseWindow => {
                    is_terminated.store(true, std::sync::atomic::Ordering::Relaxed);
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

        match read_key_handler.join() {
            Ok(ret) => ret?,
            Err(e) => {
                if let Some(e) = e.downcast_ref::<&str>() {
                    panic!("read_key thread panicked: {:?}", e);
                };
            }
        }

        match kube_process_handler.join() {
            Ok(ret) => ret?,
            Err(e) => {
                if let Some(e) = e.downcast_ref::<&str>() {
                    panic!("kube_process thread panicked: {:?}", e);
                };
            }
        }

        match tick_handler.join() {
            Ok(ret) => ret?,
            Err(e) => {
                if let Some(e) = e.downcast_ref::<&str>() {
                    panic!("tick thread panicked: {:?}", e);
                };
            }
        }

        Ok(())
    }
}
